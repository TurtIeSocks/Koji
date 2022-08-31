pub fn count(points: Vec<[f64; 2]>, clusters: Vec<[f64; 2]>, min: i32) -> Vec<[f64; 2]> {
    let mut filtered_clusters: Vec<[f64; 2]> = Vec::new();

    for [cx, cy] in clusters.iter() {
        let mut count: i32 = 0;
        for [x, y] in points.iter() {
            let dx = x - cx;
            let dy = y - cy;
            let d = dx * dx + dy * dy;
            if d < 1. {
                count += 1;
            }
        }
        if count >= min {
            filtered_clusters.push([*cx, *cy]);
        }
    }

    filtered_clusters
}
