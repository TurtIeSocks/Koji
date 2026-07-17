//! Clustering benchmark harness.
//!
//! Runs the production clustering entry point (`clustering::main`) over seeded
//! synthetic datasets and reports mygod_score + timings as machine-greppable
//! RESULT lines.
//!
//! Usage:
//!   cargo run --release -p algorithms --bin clusterbench -- \
//!     --dataset urban --n 10000 --mode best --min-points 1 [--legacy] [--seed 42]
//!
//! Datasets: uniform | blobs | urban | csv:<path>
//! Modes: fastest | honeycomb | fast | balanced | better | best

use web_time::Instant;

use algorithms::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
use algorithms::{clustering, stats::Stats};
use geojson::FeatureCollection;
use koji_core::{Precision, SingleVec};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

const CENTER_LAT: Precision = 40.7128;
const CENTER_LON: Precision = -74.0060;
const KM_PER_DEG_LAT: Precision = 111.32;

struct Args {
    dataset: String,
    n: usize,
    mode: String,
    min_points: usize,
    max_clusters: usize,
    radius: Precision,
    seed: u64,
    legacy: bool,
    /// After the cold run: churn the input, then compare a fresh cold run
    /// against `Crucible::run_seeded` warm-started from the pre-churn solution.
    warm: bool,
    churn_pct: Precision,
    /// Sweep KOJI_SCORE_LAMBDA_OVERLAP over a grid and print the
    /// (score, overlap) Pareto frontier instead of a single run. Route and
    /// knife lambdas are report-only weights, so only overlap is swept.
    sweep: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        dataset: "uniform".to_string(),
        n: 10_000,
        mode: "balanced".to_string(),
        min_points: 1,
        max_clusters: usize::MAX,
        radius: 70.0,
        seed: 42,
        legacy: false,
        warm: false,
        churn_pct: 5.0,
        sweep: false,
    };
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        let mut take = |key: &str| -> Option<String> {
            if argv[i] == key {
                let v = argv.get(i + 1).cloned();
                i += 1;
                v
            } else {
                None
            }
        };
        if let Some(v) = take("--dataset") {
            args.dataset = v;
        } else if let Some(v) = take("--n") {
            args.n = v.parse().expect("--n must be an integer");
        } else if let Some(v) = take("--mode") {
            args.mode = v;
        } else if let Some(v) = take("--min-points") {
            args.min_points = v.parse().expect("--min-points must be an integer");
        } else if let Some(v) = take("--max-clusters") {
            args.max_clusters = v.parse().expect("--max-clusters must be an integer");
        } else if let Some(v) = take("--radius") {
            args.radius = v.parse().expect("--radius must be a float");
        } else if let Some(v) = take("--seed") {
            args.seed = v.parse().expect("--seed must be an integer");
        } else if let Some(v) = take("--churn") {
            args.churn_pct = v.parse().expect("--churn must be a float pct");
        } else if argv[i] == "--legacy" {
            args.legacy = true;
        } else if argv[i] == "--warm" {
            args.warm = true;
        } else if argv[i] == "--sweep" {
            args.sweep = true;
        } else {
            panic!("unknown arg: {}", argv[i]);
        }
        i += 1;
    }
    args
}

fn km_to_deg_lat(km: Precision) -> Precision {
    km / KM_PER_DEG_LAT
}

fn km_to_deg_lon(km: Precision) -> Precision {
    km / (KM_PER_DEG_LAT * CENTER_LAT.to_radians().cos())
}

/// Box-Muller standard normal.
fn randn(rng: &mut SmallRng) -> Precision {
    let u1: Precision = rng.random::<Precision>().max(1e-12);
    let u2: Precision = rng.random();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn uniform_in_box(rng: &mut SmallRng, side_km: Precision) -> [Precision; 2] {
    let half_lat = km_to_deg_lat(side_km) / 2.0;
    let half_lon = km_to_deg_lon(side_km) / 2.0;
    [
        CENTER_LAT + (rng.random::<Precision>() * 2.0 - 1.0) * half_lat,
        CENTER_LON + (rng.random::<Precision>() * 2.0 - 1.0) * half_lon,
    ]
}

/// Uniform points at ~100 pts/km² (≈1.5 points per 70 m disk).
fn gen_uniform(n: usize, seed: u64) -> SingleVec {
    let mut rng = SmallRng::seed_from_u64(seed);
    let side_km = (n as Precision / 100.0).sqrt().max(0.2);
    (0..n).map(|_| uniform_in_box(&mut rng, side_km)).collect()
}

/// Gaussian POI blobs (σ 25–120 m) + 15% uniform background.
fn gen_blobs(n: usize, seed: u64) -> SingleVec {
    let mut rng = SmallRng::seed_from_u64(seed);
    let side_km = (n as Precision / 150.0).sqrt().max(0.2);
    let n_blobs = (n / 40).max(3);
    let blobs: Vec<([Precision; 2], Precision)> = (0..n_blobs)
        .map(|_| {
            let c = uniform_in_box(&mut rng, side_km);
            let sigma_m = 25.0 + rng.random::<Precision>() * 95.0;
            (c, sigma_m)
        })
        .collect();
    (0..n)
        .map(|_| {
            if rng.random::<Precision>() < 0.15 {
                uniform_in_box(&mut rng, side_km)
            } else {
                let (c, sigma_m) = blobs[rng.random_range(0..n_blobs)];
                let sigma_km = sigma_m / 1000.0;
                [
                    c[0] + km_to_deg_lat(randn(&mut rng) * sigma_km),
                    c[1] + km_to_deg_lon(randn(&mut rng) * sigma_km),
                ]
            }
        })
        .collect()
}

/// Street-grid lattice (120 m spacing, ±15 m jitter) + downtown blob + noise.
fn gen_urban(n: usize, seed: u64) -> SingleVec {
    let mut rng = SmallRng::seed_from_u64(seed);
    let side_km = (n as Precision / 250.0).sqrt().max(0.2);
    let spacing_km = 0.12;
    let n_streets = (side_km / spacing_km).floor() as i64;
    let jitter_km = 0.015;
    (0..n)
        .map(|_| {
            let roll: Precision = rng.random();
            if roll < 0.70 {
                // on-street point
                let street = rng.random_range(0..n_streets) as Precision;
                let along = (rng.random::<Precision>() - 0.5) * side_km;
                let perp = (street - n_streets as Precision / 2.0) * spacing_km
                    + randn(&mut rng) * jitter_km;
                if rng.random::<bool>() {
                    [
                        CENTER_LAT + km_to_deg_lat(perp),
                        CENTER_LON + km_to_deg_lon(along),
                    ]
                } else {
                    [
                        CENTER_LAT + km_to_deg_lat(along),
                        CENTER_LON + km_to_deg_lon(perp),
                    ]
                }
            } else if roll < 0.90 {
                // downtown gaussian
                [
                    CENTER_LAT + km_to_deg_lat(randn(&mut rng) * 0.5),
                    CENTER_LON + km_to_deg_lon(randn(&mut rng) * 0.5),
                ]
            } else {
                uniform_in_box(&mut rng, side_km)
            }
        })
        .collect()
}

fn load_csv(path: &str) -> SingleVec {
    let contents =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    contents
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.split(',');
            let lat = parts.next()?.trim().parse::<Precision>().ok()?;
            let lon = parts.next()?.trim().parse::<Precision>().ok()?;
            Some([lat, lon])
        })
        .collect()
}

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn main() {
    let args = parse_args();
    if std::env::var("BENCH_VERBOSE").is_ok() {
        let _ = log::set_logger(&LOGGER).map(|_| log::set_max_level(log::LevelFilter::Info));
    }

    // Note: --legacy flag is kept in the report output only; the legacy greedy
    // path is no longer available (all quality modes route to crucible).
    let points: SingleVec = if let Some(path) = args.dataset.strip_prefix("csv:") {
        let mut p = load_csv(path);
        if args.n > 0 && args.n < p.len() {
            p.truncate(args.n);
        }
        p
    } else {
        match args.dataset.as_str() {
            "uniform" => gen_uniform(args.n, args.seed),
            "blobs" => gen_blobs(args.n, args.seed),
            "urban" => gen_urban(args.n, args.seed),
            other => panic!("unknown dataset: {other}"),
        }
    };

    let mode = match args.mode.as_str() {
        "fastest" => ClusterMode::Fastest,
        "honeycomb" => ClusterMode::Honeycomb,
        "fast" => ClusterMode::Fast,
        "balanced" => ClusterMode::Balanced,
        "better" => ClusterMode::Better,
        "best" => ClusterMode::Best,
        other => panic!("unknown mode: {other}"),
    };

    let cfg = ClusteringConfig {
        mode,
        radius: args.radius,
        min_points: args.min_points,
        max_clusters: args.max_clusters,
        calculation_mode: CalculationMode::Radius,
        s2: S2Config { level: 15, size: 1 },
        center_clusters: false,
        plugin_args: String::new(),
    };

    // Lambda sweep: re-run the full pipeline per overlap weight and report
    // the (score, overlap_excess) Pareto frontier.
    if args.sweep {
        let mut rows: Vec<(&str, usize, usize, usize)> = Vec::new();
        for lambda in ["0", "0.25", "0.5", "1", "2", "4"] {
            // Set before the run's worker threads exist; read once per run
            // by the refiner and the scorer.
            unsafe { std::env::set_var("KOJI_SCORE_LAMBDA_OVERLAP", lambda) };
            let mut stats = Stats::new(format!("bench-sweep-{lambda}"), args.min_points);
            let wall = Instant::now();
            let clusters = clustering::main(
                &points,
                &cfg,
                FeatureCollection {
                    bbox: None,
                    features: vec![],
                    foreign_members: None,
                },
                &mut stats,
            );
            report(
                &args,
                &format!("sweep-L{lambda}"),
                &points,
                &clusters,
                &stats,
                wall.elapsed().as_secs_f64(),
            );
            rows.push((
                lambda,
                stats.mygod_score,
                stats.score_components.overlap_excess,
                stats.score_components.knife_edge,
            ));
        }
        for (i, (l, s, e, k)) in rows.iter().enumerate() {
            let dominated = rows
                .iter()
                .enumerate()
                .any(|(j, (_, s2, e2, _))| j != i && s2 <= s && e2 <= e && (s2 < s || e2 < e));
            println!(
                "PARETO lambda={l} score={s} excess={e} knife={k}{}",
                if dominated { "" } else { " frontier" }
            );
        }
        return;
    }

    let mut stats = Stats::new(format!("bench-{}", args.mode), args.min_points);
    let wall = Instant::now();
    let clusters = clustering::main(
        &points,
        &cfg,
        FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        },
        &mut stats,
    );
    let wall_s = wall.elapsed().as_secs_f64();
    report(&args, "main", &points, &clusters, &stats, wall_s);

    // Warm-start comparison: churn the input, then run cold vs seeded on the
    // same churned points. The seed is the pre-churn solution — the
    // incremental re-cluster case Koji hits as spawnpoints come and go.
    if args.warm {
        let churned = churn(&points, args.churn_pct, args.seed);
        let mut stats_c = Stats::new("bench-cold-churn".to_string(), args.min_points);
        let wall_c = Instant::now();
        let cold = clustering::main(
            &churned,
            &cfg,
            FeatureCollection {
                bbox: None,
                features: vec![],
                foreign_members: None,
            },
            &mut stats_c,
        );
        report(
            &args,
            "cold-churn",
            &churned,
            &cold,
            &stats_c,
            wall_c.elapsed().as_secs_f64(),
        );

        let crucible = clustering::Crucible {
            radius: args.radius,
            min_points: args.min_points,
            max_clusters: usize::MAX,
            quick: false,
        };
        let mut stats_w = Stats::new("bench-warm-churn".to_string(), args.min_points);
        let wall_w = Instant::now();
        let cluster_timer = Instant::now();
        let warm = crucible.run_seeded(&churned, &clusters);
        stats_w.set_cluster_time(cluster_timer);
        stats_w.cluster_stats(args.radius, &churned, &warm);
        stats_w.set_score();
        report(
            &args,
            "warm-churn",
            &churned,
            &warm,
            &stats_w,
            wall_w.elapsed().as_secs_f64(),
        );
    }
}

/// Deterministic ±pct% churn: drop every k-th point, add the same count of
/// new points jittered up to ~±220 m around random survivors.
fn churn(points: &SingleVec, pct: Precision, seed: u64) -> SingleVec {
    let mut rng = SmallRng::seed_from_u64(seed ^ 0x4348_5552); // "CHUR"
    let k = (100.0 / pct.max(0.01)).round().max(1.0) as usize;
    let mut out: SingleVec = points
        .iter()
        .enumerate()
        .filter(|(i, _)| i % k != 0)
        .map(|(_, p)| *p)
        .collect();
    let n_add = points.len() / k;
    for _ in 0..n_add {
        let base = points[rng.random_range(0..points.len())];
        out.push([
            base[0] + (rng.random::<Precision>() - 0.5) * 0.004,
            base[1] + (rng.random::<Precision>() - 0.5) * 0.004,
        ]);
    }
    out
}

fn report(
    args: &Args,
    phase: &str,
    points: &SingleVec,
    clusters: &SingleVec,
    stats: &Stats,
    wall_s: Precision,
) {
    let lb = if args.min_points == 1 {
        algorithms::stats::independent_set_lb(points, args.radius)
    } else {
        0
    };
    println!(
        "RESULT dataset={} n={} mode={} legacy={} min_points={} radius={} seed={} clusters={} covered={} total={} score={} lb={} route_m={:.0} route_s={:.0} knife={} multi={} excess={} quality={:.3} cluster_s={:.2} wall_s={:.2} phase={}",
        args.dataset,
        points.len(),
        args.mode,
        args.legacy,
        args.min_points,
        args.radius,
        args.seed,
        clusters.len(),
        stats.points_covered,
        stats.total_points,
        stats.mygod_score,
        lb,
        stats.score_components.route_est_m,
        stats.score_components.route_est_s,
        stats.score_components.knife_edge,
        stats.score_components.multi_covered,
        stats.score_components.overlap_excess,
        stats.score_components.quality,
        stats.cluster_time,
        wall_s,
        phase,
    );
}
