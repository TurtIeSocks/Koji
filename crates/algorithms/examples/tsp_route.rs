//! Real-data routing harness: cluster a file of `lat,lon` points with crucible
//! (min_points = 1, radius = 70 m), then route the cluster centers with both
//! the S2 seed sort and the `SortBy::Tsp` refiner, and compare. Pure compute,
//! no database.
//!
//! Run: `cargo run --release --example tsp_route -- points.csv`
//!
//! Input: one point per line, `lat,lon` (comma- or whitespace-separated). A
//! header row, blanks, and any non-coordinate rows are skipped. Accepts `.csv`
//! or `.txt` (any extension, really). Output is written next to the input as
//! `<stem>_out.<ext>` (route order) plus `<stem>_route.svg` (before/after map).

use std::path::{Path, PathBuf};
use koji_core::Precision;
use std::time::Instant;

use algorithms::clustering::{self, CalculationMode, ClusterMode, ClusteringConfig, S2Config};
use algorithms::routing::{self, RoutingConfig, SortBy};
use algorithms::stats::Stats;
use geojson::FeatureCollection;
use koji_core::SingleVec;

const RADIUS: Precision = 70.0;

/// Great-circle metres between two `[lat, lon]` points.
fn haversine_m(a: [Precision; 2], b: [Precision; 2]) -> Precision {
    let r = 6_371_000.0_f64;
    let (lat1, lat2) = (a[0].to_radians(), b[0].to_radians());
    let dlat = (b[0] - a[0]).to_radians();
    let dlon = (b[1] - a[1]).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

/// (total cyclic length, closing-leg length) in metres.
fn metrics(order: &[[Precision; 2]]) -> (Precision, Precision) {
    let n = order.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let total: Precision = (0..n).map(|i| haversine_m(order[i], order[(i + 1) % n])).sum();
    (total, haversine_m(order[n - 1], order[0]))
}

/// Parse `lat,lon` rows. Splits on comma or whitespace; keeps only rows whose
/// first two fields are numbers in valid lat/lon range — so a header row, blank
/// lines, and stray text are all dropped.
fn parse_points(raw: &str) -> SingleVec {
    raw.lines()
        .filter_map(|line| {
            let mut it = line
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty());
            let lat = it.next()?.parse::<Precision>().ok()?;
            let lon = it.next()?.parse::<Precision>().ok()?;
            (lat.abs() <= 90.0 && lon.abs() <= 180.0).then_some([lat, lon])
        })
        .collect()
}

/// Derive a sibling output path: `<stem><suffix>.<ext>` next to the input.
/// `ext` overrides the input's extension when set (e.g. the SVG).
fn derive_output(input: &str, suffix: &str, ext: Option<&str>) -> PathBuf {
    let p = Path::new(input);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    let ext = ext
        .map(str::to_string)
        .or_else(|| p.extension().map(|e| e.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "csv".into());
    let name = format!("{stem}{suffix}.{ext}");
    match p.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(name),
        _ => PathBuf::from(name),
    }
}

fn route(clusters: &SingleVec, sort_by: SortBy) -> (SingleVec, std::time::Duration) {
    let cfg = RoutingConfig {
        sort_by,
        plugin_args: String::new(),
    };
    let mut stats = Stats::new("route".to_string(), 1);
    let t = Instant::now();
    let out = routing::main(&vec![], clusters.clone(), RADIUS, &cfg, &mut stats);
    (out, t.elapsed())
}

fn main() {
    let input = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: tsp_route <input.csv|.txt>   (lat,lon per row; headers skipped)");
            std::process::exit(2);
        }
    };

    // ── 1. Load + parse the input ────────────────────────────────────────────
    let raw = std::fs::read_to_string(&input).unwrap_or_else(|e| {
        eprintln!("error: cannot read {input}: {e}");
        std::process::exit(1);
    });
    let data = parse_points(&raw);
    println!("input: {input} — {} points", data.len());
    if data.len() < 4 {
        eprintln!("error: need at least 4 valid lat,lon rows");
        std::process::exit(1);
    }

    // ── 2. Cluster with crucible (Better → crucible), m=1, r=70 ──────────────
    let cfg = ClusteringConfig {
        mode: ClusterMode::Better,
        radius: RADIUS,
        min_points: 1,
        max_clusters: usize::MAX,
        calculation_mode: CalculationMode::Radius,
        s2: S2Config::default(),
        center_clusters: false,
        genetic_post_processing: false,
        plugin_args: String::new(),
    };
    let mut stats = Stats::new("tsp_route".to_string(), 1);
    let t = Instant::now();
    let clusters = clustering::main(&data, &cfg, FeatureCollection::default(), &mut stats);
    println!(
        "crucible clusters: {} (in {:.2}s, radius {RADIUS} m, min_points 1)",
        clusters.len(),
        t.elapsed().as_secs_f64()
    );

    // ── 3. Route: S2 seed vs the Tsp refiner ─────────────────────────────────
    let (s2_order, s2_dur) = route(&clusters, SortBy::S2Cell);
    let (tsp_order, tsp_dur) = route(&clusters, SortBy::Tsp);
    let (s2_total, s2_close) = metrics(&s2_order);
    let (tsp_total, tsp_close) = metrics(&tsp_order);

    let km = |m: Precision| m / 1000.0;
    println!("\n──────────── routing comparison ({} clusters) ────────────", clusters.len());
    println!("                     S2 seed (s2cell)      Tsp (2-opt+Or-opt)");
    println!(
        "total tour          {:>12.2} km     {:>12.2} km   ({:+.1}%)",
        km(s2_total),
        km(tsp_total),
        100.0 * (tsp_total - s2_total) / s2_total
    );
    println!(
        "closing leg         {:>12.0} m      {:>12.0} m    ({:+.1}%)",
        s2_close,
        tsp_close,
        100.0 * (tsp_close - s2_close) / s2_close
    );
    println!(
        "compute time        {:>12.2}s       {:>12.2}s",
        s2_dur.as_secs_f64(),
        tsp_dur.as_secs_f64()
    );

    // ── 4. Write outputs next to the input ───────────────────────────────────
    let out_csv = derive_output(&input, "_out", None);
    let body: String = tsp_order
        .iter()
        .map(|p| format!("{:.8},{:.8}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&out_csv, body).expect("write route csv");
    println!("\nTsp route ({} points) → {}", tsp_order.len(), out_csv.display());

    let out_svg = derive_output(&input, "_route", Some("svg"));
    let svg = render_svg(&s2_order, &tsp_order, s2_total, s2_close, tsp_total, tsp_close);
    std::fs::write(&out_svg, svg).expect("write svg");
    println!("route map → {}", out_svg.display());
}

/// Two-panel before/after route map. Equirectangular projection (aspect
/// preserved); the closing leg (last→first) is drawn bold so a long return is
/// visible against the Tsp tour's tight one.
fn render_svg(
    s2: &SingleVec,
    tsp: &SingleVec,
    s2_total: Precision,
    s2_close: Precision,
    tsp_total: Precision,
    tsp_close: Precision,
) -> String {
    let lat_min = s2.iter().map(|p| p[0]).fold(Precision::MAX, Precision::min);
    let lat_max = s2.iter().map(|p| p[0]).fold(Precision::MIN, Precision::max);
    let lon_min = s2.iter().map(|p| p[1]).fold(Precision::MAX, Precision::min);
    let lon_max = s2.iter().map(|p| p[1]).fold(Precision::MIN, Precision::max);
    let mean_lat = (0.5 * (lat_min + lat_max)).to_radians();
    let inner_w = 282.0_f64;
    let geo_w = ((lon_max - lon_min) * mean_lat.cos()).max(1e-9);
    let scale = inner_w / geo_w;
    let inner_h = (lat_max - lat_min) * scale;
    let top = 70.0_f64;
    let height = top + inner_h + 30.0;

    let project = |p: &[Precision; 2], px: Precision| -> (Precision, Precision) {
        let x = px + (p[1] - lon_min) * mean_lat.cos() * scale;
        let y = top + (lat_max - p[0]) * scale;
        (x, y)
    };
    let polyline = |order: &SingleVec, px: Precision| -> String {
        order
            .iter()
            .map(|p| {
                let (x, y) = project(p, px);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let closing = |order: &SingleVec, px: Precision| -> (Precision, Precision, Precision, Precision) {
        let (x1, y1) = project(&order[order.len() - 1], px);
        let (x2, y2) = project(&order[0], px);
        (x1, y1, x2, y2)
    };

    let (lx, rx) = (24.0_f64, 374.0_f64);
    let s2_line = polyline(s2, lx);
    let tsp_line = polyline(tsp, rx);
    let (sx1, sy1, sx2, sy2) = closing(s2, lx);
    let (tx1, ty1, tx2, ty2) = closing(tsp, rx);

    format!(
        r##"<svg width="100%" viewBox="0 0 680 {height:.0}" role="img" xmlns="http://www.w3.org/2000/svg">
<title>Route: S2 seed vs Tsp refiner</title>
<desc>Before/after closed-tour routes over the cluster centers; the S2 seed's long closing leg shrinks after 2-opt/Or-opt.</desc>
<text x="24" y="28" style="font-size:15px;font-weight:500;fill:var(--color-text-primary)">S2 seed (s2cell)</text>
<text x="24" y="48" style="font-size:12px;fill:var(--color-text-secondary)">tour {s2_total_km:.0} km · closing {s2_close:.0} m</text>
<text x="374" y="28" style="font-size:15px;font-weight:500;fill:var(--color-text-primary)">Tsp (2-opt + Or-opt)</text>
<text x="374" y="48" style="font-size:12px;fill:var(--color-text-secondary)">tour {tsp_total_km:.0} km · closing {tsp_close:.0} m</text>
<polyline points="{s2_line}" fill="none" stroke="var(--color-text-tertiary)" stroke-width="0.4"/>
<line x1="{sx1:.1}" y1="{sy1:.1}" x2="{sx2:.1}" y2="{sy2:.1}" stroke="var(--color-danger)" stroke-width="2"/>
<polyline points="{tsp_line}" fill="none" stroke="var(--color-text-tertiary)" stroke-width="0.4"/>
<line x1="{tx1:.1}" y1="{ty1:.1}" x2="{tx2:.1}" y2="{ty2:.1}" stroke="var(--color-success)" stroke-width="2"/>
</svg>"##,
        s2_total_km = s2_total / 1000.0,
        tsp_total_km = tsp_total / 1000.0,
    )
}
