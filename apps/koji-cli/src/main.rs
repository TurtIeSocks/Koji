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
//! HTTP `run_calc` path). For the `calculate` kind, the payload must be a
//! fully-resolved [`koji_service::CalcPayload`] shape.
//!
//! Startup mirrors `apps/koji-server/src/main.rs`: load `.env` (override via the
//! `ENV` var) and init `env_logger` from `LOG_LEVEL`. It then bootstraps the DB
//! via [`koji_db::utils::get_database_struct`] (needs `KOJI_DB_URL` +
//! `SCANNER_DB_URL`), so it is **not** runnable without a live DB.

use std::error::Error;
use std::process;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use koji_jobs::{AwaitError, HandlerRegistry, JobId, JobOutcome, JobQueue};
use koji_service::CalculateHandler;

/// Wait budget for `enqueue --wait`, matching the HTTP sync bridge (spec §7:
/// 290s, then report the job id as "still running").
const WAIT_BUDGET: Duration = Duration::from_secs(290);

/// Default job priority for CLI enqueues. `0` is the DB default and the same
/// value the generic `POST /api/v2/jobs` endpoint uses for async work (sync calc
/// uses a higher priority; CLI/async work is normal-priority — spec §6/§8).
const PRIORITY_NORMAL: i16 = 0;

/// Operator CLI over the Koji V2 job queue.
#[derive(Debug, Parser)]
#[command(name = "koji-cli", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a job-worker pool (registers the `calculate` handler) until Ctrl-C.
    Worker {
        /// Number of concurrent workers in this process.
        #[arg(long, default_value_t = 1)]
        concurrency: usize,
    },
    /// Enqueue a job and optionally wait for its result.
    Enqueue {
        /// Handler kind to run (e.g. `calculate`).
        #[arg(long)]
        kind: String,
        /// Job payload as a JSON string, or `@path` to read JSON from a file.
        #[arg(long)]
        payload: String,
        /// Block for the result (up to ~290s) instead of returning the id.
        #[arg(long, default_value_t = false)]
        wait: bool,
        /// Job priority (`priority DESC` claim order). Defaults to normal (0).
        #[arg(long)]
        priority: Option<i16>,
    },
    /// Read a job's status / progress / result by id.
    Get {
        /// The job id (26-char ULID `public_id`).
        #[arg(long)]
        id: String,
    },
}

#[tokio::main]
async fn main() {
    // Mirror the server's startup: load `.env` (override file via `ENV`) and
    // configure `env_logger` from `LOG_LEVEL` (default `info`), to stdout.
    dotenv::from_filename(std::env::var("ENV").unwrap_or(".env".into())).ok();
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::new()
            .default_filter_or(std::env::var("LOG_LEVEL").unwrap_or("info".to_string())),
    );
    builder.target(env_logger::Target::Stdout);
    builder.init();

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
        // caller can poll it later with `get` (spec §7/§11).
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
