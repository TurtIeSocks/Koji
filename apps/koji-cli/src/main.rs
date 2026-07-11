//! # koji-cli
//!
//! A thin operator CLI over the Koji V2 job queue (`koji-jobs`). It wires the
//! same `JobQueue` the server does (`apps/koji-server`) against the Koji DB and
//! exposes three subcommands:
//!
//! - `worker`  — run a job-worker pool (registers the `calculate` handler) until
//!   Ctrl-C. Use this to drain the queue out-of-process from the HTTP server.
//! - `enqueue` — push a job (any `--kind` + `--payload` JSON) and optionally
//!   block for its result (`--wait`).
//! - `get`     — read a job's status / progress / result by id.
//!
//! It deliberately stays generic: `enqueue` takes the raw payload JSON the
//! handler expects and performs **no** calc-input resolution (that lives in the
//! HTTP `create_job` handler, `koji-service/src/public/v2/jobs.rs`). For the
//! `calculate` kind, the payload must be a fully-resolved
//! [`koji_service::CalcPayload`] shape.
//!
//! Startup mirrors `apps/koji-server/src/main.rs`: load `.env` (override via the
//! `ENV` var) and init `env_logger` from `LOG_LEVEL`. It then bootstraps the DB
//! via [`koji_db::utils::get_database_struct`] (needs `KOJI_DB_URL` +
//! `GOLBAT_DB_URL`), so it is **not** runnable without a live DB.

use std::error::Error;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use koji_jobs::{AwaitError, HandlerRegistry, JobId, JobOutcome, JobQueue};
use koji_service::CalculateHandler;

/// Wait budget for `enqueue --wait`, matching the HTTP sync bridge (v2-api
/// redesign §4.1: 290s, then report the job id as "still running").
const WAIT_BUDGET: Duration = Duration::from_secs(290);

/// Default job priority for CLI enqueues. `0` is the DB default and the same
/// value the generic `POST /api/v2/jobs` endpoint uses for async work (sync calc
/// uses a higher priority; CLI/async work is normal-priority — job-queue-design
/// §4 schema + §8 claim order).
const PRIORITY_NORMAL: i16 = 0;

/// Operator CLI over the Koji V2 job queue.
///
/// Wires the same `JobQueue` the HTTP server uses against the Koji DB, so you can
/// drive the queue out-of-process: run workers, push jobs, and read their state.
///
/// REQUIRES A LIVE DB. On startup it loads `.env` (override the file via the `ENV`
/// env var) and bootstraps the DB from `KOJI_DB_URL` + `GOLBAT_DB_URL` — every
/// subcommand, `get` included, opens the DB before doing anything. Logging uses
/// `env_logger`, reading `LOG_LEVEL` (default `info`) and writing to stdout.
#[derive(Debug, Parser)]
#[command(
    name = "koji-cli",
    version,
    after_long_help = r#"EXAMPLES:
  # Drain the queue with 4 workers until Ctrl-C
  koji-cli worker --concurrency 4

  # Enqueue a calc job from a file and block for the result
  koji-cli enqueue --kind calculate --payload @calc.json --wait

  # Enqueue async (prints {"job_id": "..."}), then poll it later
  koji-cli enqueue --kind calculate --payload @calc.json
  koji-cli get --id 01J9Z6P7QK8X3M2YQF3V8B7C9D

ENVIRONMENT:
  ENV            dotenv file to load                    [default: .env]
  KOJI_DB_URL    Koji database URL                      [required]
  GOLBAT_DB_URL  Golbat database URL                    [required]
  LOG_LEVEL      env_logger filter directive            [default: info]"#
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a job-worker pool (registers the `calculate` handler) until Ctrl-C.
    ///
    /// Registers the `calculate` handler (same as the server) and claims jobs in
    /// `priority DESC` order. Use it to process the queue out-of-process from the
    /// HTTP server; run as many as you like across processes/machines — job claims
    /// are atomic, so workers never double-run a job. On Ctrl-C it stops claiming
    /// new jobs and drains the in-flight ones before exiting.
    Worker {
        /// Number of concurrent workers in this process.
        ///
        /// Each worker runs one job at a time, so this is the per-process
        /// parallelism. Clamped to at least 1.
        #[arg(long, default_value_t = 1)]
        concurrency: usize,
    },
    /// Enqueue a job and optionally wait for its result.
    ///
    /// Generic: takes any handler `--kind` plus the raw `--payload` JSON that
    /// handler expects, and performs NO input resolution. For `--kind calculate`
    /// the payload must already be a fully-resolved `koji_service::CalcPayload`
    /// (the HTTP `POST /api/v2/jobs` path resolves area / data-points before
    /// enqueue; the CLI does not). Every CLI enqueue is its own job — no dedup
    /// (the HTTP path composes a dedup key; the CLI passes none).
    #[command(after_long_help = r#"CALC PAYLOAD (--kind calculate):
  A `CalcPayload` JSON object carrying the already-resolved inputs:
    mode         calc mode string (e.g. `fastest`, `route`, `bootstrap`)
    category     data category (`pokestop`, `fort`, `station`, `spawnpoint`)
    request      the tagged CalcRequest body (its `mode` field selects the op)
    area         pre-resolved GeoJSON FeatureCollection
    data_points  pre-resolved [[lat, lon], ...] (may be empty for bootstrap)
    clusters     pre-resolved cluster set for reroute / route-stats (optional)
  This is the resolved shape, NOT the HTTP request body. See
  `koji_service::CalcPayload` / the server OpenAPI schema for the exact,
  current field set.

EXAMPLES:
  koji-cli enqueue --kind calculate --payload @calc.json --wait
  koji-cli enqueue --kind calculate --payload '{"mode":"fastest","category":"pokestop", ...}'"#)]
    Enqueue {
        /// Handler kind to run (e.g. `calculate`).
        ///
        /// Must match a handler registered by a running `worker` (or the server).
        /// The calc handler's kind is `calculate`; an unknown kind enqueues a job
        /// no worker will ever claim.
        #[arg(long)]
        kind: String,
        /// Job payload as a JSON string, or `@path` to read JSON from a file.
        ///
        /// A leading `@` reads the JSON from the named file (`--payload @calc.json`);
        /// otherwise the argument itself is parsed as JSON. Must match the shape the
        /// target handler expects — see the CALC PAYLOAD notes for `--kind calculate`.
        #[arg(long)]
        payload: String,
        /// Block for the result (up to ~290s) instead of returning the id.
        ///
        /// Without `--wait`, prints `{"job_id": "..."}` immediately. With `--wait`,
        /// blocks for a terminal outcome: on success prints the result JSON (exit 0);
        /// on job failure/cancel prints the error (exit 1). If the ~290s budget
        /// elapses the job keeps running and it prints
        /// `{"job_id": "...", "status": "still running"}` (exit 0) — poll it with `get`.
        #[arg(long, default_value_t = false)]
        wait: bool,
        /// Job priority; workers claim highest-first (`priority DESC`).
        ///
        /// Defaults to normal (0) — the same priority async HTTP work gets. Raise it
        /// to jump ahead of normal jobs in the claim order.
        #[arg(long)]
        priority: Option<i16>,
    },
    /// Read a job's status / progress / result by id.
    ///
    /// Prints the job's observable state as pretty JSON: id, kind, status, progress,
    /// phase, result, and error. If no job has that id, prints
    /// `{"error": "not found"}` and exits 0.
    Get {
        /// The job id — a 26-character ULID (`public_id`).
        ///
        /// This is the `job_id` printed by `enqueue`.
        #[arg(long)]
        id: String,
    },
}

#[tokio::main]
async fn main() {
    koji_service::init_env_and_logging();

    if let Err(err) = run().await {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Bootstrap the DB exactly as the server does, then build the queue over the
    // Koji connection. The worker id namespaces this process in `job.locked_by`.
    let db = koji_db::utils::get_database_struct().await;
    let jobs = Arc::new(JobQueue::new(
        db.koji.clone(),
        format!("koji-cli-{}", process::id()),
    ));

    match cli.command {
        Command::Worker { concurrency } => worker(db, jobs, concurrency).await,
        Command::Enqueue {
            kind,
            payload,
            wait,
            priority,
        } => enqueue(jobs, kind, payload, wait, priority).await,
        Command::Get { id } => get(jobs, id).await,
    }
}

/// Run a worker pool until Ctrl-C, then shut down gracefully.
///
/// `CalculateHandler::new` takes the whole `KojiDb` (per the architecture: the
/// handler holds a DB handle for future phases), so this hands it `db` directly
/// — same as `koji_service::start()`.
async fn worker(
    db: koji_db::KojiDb,
    jobs: Arc<JobQueue>,
    concurrency: usize,
) -> Result<(), Box<dyn Error>> {
    let registry = HandlerRegistry::new().register(CalculateHandler::new(db));
    // `spawn_workers` clamps to >= 1; bind the set so the tasks stay alive for
    // the lifetime of this call (dropping it would just detach them).
    let workers = Arc::clone(&jobs).spawn_workers(concurrency, registry);
    log::info!(
        "[koji-cli] spawned {} job worker(s); waiting for Ctrl-C",
        workers.len()
    );

    tokio::signal::ctrl_c().await?;
    log::info!("[koji-cli] shutdown requested; draining in-flight jobs");
    workers.shutdown().await;
    log::info!("[koji-cli] workers stopped");
    Ok(())
}

/// Enqueue a job (no dedup) and, with `--wait`, block for its terminal outcome.
async fn enqueue(
    jobs: Arc<JobQueue>,
    kind: String,
    payload: String,
    wait: bool,
    priority: Option<i16>,
) -> Result<(), Box<dyn Error>> {
    let payload = parse_payload(&payload)?;
    let priority = priority.unwrap_or(PRIORITY_NORMAL);

    // No dedup from the CLI: every enqueue is its own job (the dedup identity is
    // composed in the HTTP calc path, not here).
    let id = jobs
        .enqueue_or_attach(&kind, None, &payload, priority)
        .await?;

    if !wait {
        println!("{}", serde_json::json!({ "job_id": id.as_string() }));
        return Ok(());
    }

    // Sync bridge: block up to the wait budget for a terminal outcome, mapping it
    // to a readable result (mirrors the HTTP sync-bridge mapping).
    match jobs.await_result(id, WAIT_BUDGET).await {
        Ok(JobOutcome::Succeeded(result)) => {
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
        Ok(JobOutcome::Failed { error, code }) => {
            eprintln!("job failed [{code}]: {error}");
            process::exit(1);
        }
        Ok(JobOutcome::Canceled) => {
            eprintln!("job was canceled");
            process::exit(1);
        }
        // Timeout is NOT a failure: the job keeps running. Report the id so the
        // caller can poll it later with `get` (v2-api redesign §4.1).
        Err(AwaitError::Timeout) => {
            println!(
                "{}",
                serde_json::json!({ "job_id": id.as_string(), "status": "still running" })
            );
            Ok(())
        }
        Err(e) => Err(Box::new(e)),
    }
}

/// Read and print a job's observable state as pretty JSON.
async fn get(jobs: Arc<JobQueue>, id: String) -> Result<(), Box<dyn Error>> {
    let id: JobId = id
        .parse()
        .map_err(|_| format!("not a valid job id: {id:?}"))?;

    match jobs.get(id).await {
        // `JobRecord` is `Serialize` (id/kind/status/progress/phase/result/error).
        Ok(record) => {
            println!("{}", serde_json::to_string_pretty(&record)?);
            Ok(())
        }
        Err(AwaitError::NotFound) => {
            println!("{}", serde_json::json!({ "error": "not found" }));
            Ok(())
        }
        Err(e) => Err(Box::new(e)),
    }
}

/// Resolve a `--payload` argument to a JSON value: a leading `@` reads the JSON
/// from the named file; otherwise the argument itself is parsed as JSON.
fn parse_payload(arg: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let raw = if let Some(path) = arg.strip_prefix('@') {
        std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read payload file {path:?}: {e}"))?
    } else {
        arg.to_string()
    };
    let value = serde_json::from_str(&raw).map_err(|e| format!("invalid payload JSON: {e}"))?;
    Ok(value)
}
