use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
#[cfg(feature = "native")]
use std::fs::{File, create_dir_all};
#[cfg(feature = "native")]
use std::io::Write;

#[cfg(feature = "native")]
use colored::Colorize;
use geo::Coord;
use geohash::encode;
use hashbrown::HashSet;
use koji_core::Precision;
use koji_core::{PointArray, SingleVec};

use crate::rtree::cluster::Cluster;
use crate::stats::Stats;

#[cfg(feature = "native")]
pub fn debug_hashmap<T, U>(file_name: &str, input: &T) -> std::io::Result<()>
where
    U: Debug,
    T: Debug + Clone + IntoIterator<Item = U>,
{
    create_dir_all("./debug_files")?;
    let path = format!("./debug_files/{}", file_name);
    let mut content: String = "".to_string();

    for x in input.clone().into_iter() {
        content = format!("{}\n{:?}\n", content, x);
    }
    content = content.trim_end_matches(",").to_string();
    let mut output = File::create(path)?;
    write!(output, "{}", content)?;
    // log::info!("Saved {} to file with {} coords", file_name, input.len());
    Ok(())
}

#[cfg(feature = "native")]
pub fn debug_string(file_name: &str, input: &String) -> std::io::Result<()> {
    create_dir_all("./debug_files")?;
    let path = format!("./debug_files/{}", file_name);
    let mut output = File::create(path)?;
    write!(output, "{}", input)?;
    // log::info!("Saved {} to file with {} coords", file_name, input.len());
    Ok(())
}

pub fn get_sorted<T>(map: &HashMap<String, T>) -> Vec<(String, T)>
where
    T: Clone,
{
    let mut vec: Vec<&String> = map.keys().collect();
    vec.sort();
    vec.into_iter()
        .map(|k| (k.clone(), map.get(k).unwrap().clone()))
        .collect()
}

pub fn centroid(coords: &[[Precision; 2]]) -> PointArray {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);

    for loc in coords.iter() {
        let lat = loc[0].to_radians();
        let lon = loc[1].to_radians();

        x += lat.cos() * lon.cos();
        y += lat.cos() * lon.sin();
        z += lat.sin();
    }

    let number_of_locations = coords.len() as Precision;
    x /= number_of_locations;
    y /= number_of_locations;
    z /= number_of_locations;

    let hyp = (x * x + y * y).sqrt();
    let lon = y.atan2(x);
    let lat = z.atan2(hyp);

    [lat.to_degrees(), lon.to_degrees()]
}

#[cfg(feature = "native")]
pub fn info_log(file_name: &str, message: String) -> String {
    format!(
        "\r{}{}Z {}  {}{} {}",
        "[".black(),
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        "INFO".green(),
        file_name,
        "]".black(),
        message
    )
}

#[cfg(feature = "native")]
pub fn _debug_clusters(clusters: &HashSet<Cluster>, file_suffix: &str) {
    let mut point_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut cluster_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut unique_map: HashMap<String, HashSet<String>> = HashMap::new();

    for cluster in clusters.iter() {
        cluster_map.insert(
            cluster.point._get_geohash(),
            cluster.all.iter().map(|p| p._get_geohash()).collect(),
        );
        unique_map.insert(
            cluster.point._get_geohash(),
            cluster.unique.iter().map(|p| p._get_geohash()).collect(),
        );
        for point in cluster.all.iter() {
            point_map
                .entry(point._get_geohash())
                .and_modify(|f| {
                    f.insert(cluster.point._get_geohash());
                })
                .or_insert_with(|| {
                    let mut set: HashSet<String> = HashSet::new();
                    set.insert(cluster.point._get_geohash());
                    set
                });
        }
    }

    debug_hashmap(&format!("{}_point.txt", file_suffix), &point_map).unwrap();
    debug_hashmap(&format!("{}_cluster.txt", file_suffix), &cluster_map).unwrap();
    debug_hashmap(&format!("{}_unique.txt", file_suffix), &unique_map).unwrap();
}

pub fn rotate_to_best(clusters: SingleVec, stats: &Stats) -> SingleVec {
    let mut final_clusters = VecDeque::<PointArray>::new();

    let best_cluster_set = stats
        .best_clusters
        .iter()
        .map(|x| encode(Coord { x: x[1], y: x[0] }, 12).unwrap())
        .collect::<HashSet<String>>();
    let mut rotate_count = 0;
    for (i, [lat, lon]) in clusters.into_iter().enumerate() {
        if best_cluster_set.contains(&encode(Coord { x: lon, y: lat }, 12).unwrap()) {
            rotate_count = i;
            log::debug!("Found Best! {}, {} - {}", lat, lon, i);
        }
        final_clusters.push_back([lat, lon]);
    }
    final_clusters.rotate_left(rotate_count);

    final_clusters.into()
}
