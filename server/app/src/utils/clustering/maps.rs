use super::*;

pub fn run(
    points: &Vec<GenericData>,
    honeycomb: SingleVec,
    radius: f64,
    min_points: usize,
) -> (HashMap<String, PointInfo>, HashMap<String, CircleInfo>) {
    let time = Instant::now();

    // HashMap of approximate points using trimmed geohashes
    let mut approx_map = HashMap::<String, Vec<(String, Coord)>>::new();
    // Set of seen points when using approximate geohashes
    let mut seen_set = HashSet::<String>::new();

    // Hashmap of points and each of the circles they belong in
    // x: lon, y: lat
    let mut point_map: HashMap<String, PointInfo> = HashMap::new();
    let point_total = points.len();

    points.into_iter().for_each(|x| {
        // Flip & create the coord
        let coord = Coord {
            x: x.p[1],
            y: x.p[0],
        };
        // Precise Geohash
        let point_key = encode(coord, PRECISION).unwrap();
        // Approximate Geohash
        let approx_key = encode(coord, APPROX_PRECISION).unwrap();

        // Insert into master point map
        point_map
            .entry(point_key.clone())
            .and_modify(|info| {
                info.points += 1;
            })
            .or_insert(PointInfo {
                points: 1,
                coord,
                circles: HashSet::new(),
            });
        // Insert into approx map
        approx_map
            .entry(approx_key)
            .and_modify(|x| x.push((point_key.clone(), coord)))
            .or_insert(vec![(point_key, coord)]);
    });
    println!("Point Map Made in: {}s", time.elapsed().as_secs_f32());
    println!(
        "Points Total: {} | Consolidated: {}",
        point_total,
        point_map.len(),
    );
    println!(
        "Approx Hashes: {} | Approx Total Points: {}",
        approx_map.len(),
        approx_map.values().fold(0, |acc, x| acc + x.len())
    );

    // Hashmap of the circles from the bootstrap generator
    // x: lon, y: lat
    let circle_total = honeycomb.len();

    let mut circle_map: HashMap<_, _> = honeycomb
        .into_iter()
        .map(|x| {
            // Flip & create the coord
            let coord = Coord { x: x[1], y: x[0] };
            // Precise Geohash
            let circle_key = encode(coord, PRECISION).unwrap();
            // Approximate Geohash
            let approx_key = encode(coord, APPROX_PRECISION).unwrap();
            // Circle's bbox
            let mut bbox = BBox::new(None);

            let points: HashSet<String> = if let Some(approx_points) = approx_map.get(&approx_key) {
                // Get the points from the approx geohash
                approx_points
                    .into_iter()
                    .filter_map(|(point_key, point_coord)| {
                        // Check if the point is actually within the radius of the precise circle
                        if coord.vincenty_inverse(&point_coord) <= radius {
                            // Mark point as seen
                            seen_set.insert(encode(point_coord.clone(), PRECISION).unwrap());
                            // Update circle's bbox
                            bbox.update(*point_coord);

                            point_map
                                .entry(point_key.clone())
                                .and_modify(|mut_point_info| {
                                    mut_point_info.circles.insert(circle_key.clone());
                                });

                            // Insert the point
                            Some(point_key.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                HashSet::new()
            };
            // Insert the circle
            (
                circle_key,
                CircleInfo {
                    meets_min: points.len() >= min_points,
                    unique: HashSet::new(),
                    coord,
                    bbox,
                    points,
                },
            )
        })
        .collect();
    println!("Circle Map Made in: {}s", time.elapsed().as_secs_f32());

    println!(
      "Pre Check:\nTotal: {} | Consolidated: {} | Circle Checks: {} | Point Checks: {}\nPoints: {} / {}",
      circle_total,
      circle_map.len(),
      circle_map.values().fold(0, |acc, x| acc + x.points.len()),
      point_map.values().fold(0, |acc, x| acc + x.circles.len()),
      seen_set.len(),
      point_map.len(),
  );

    // Loops through points and adds any that were missed by the approx geohashing
    let mut factor = 1;
    while seen_set.len() != point_map.len() && factor <= APPROX_PRECISION {
        println!("Using precision... {}", APPROX_PRECISION + factor);
        for (point_key, point_info) in point_map.clone().into_iter() {
            if seen_set.contains(&point_key) {
                continue;
            }
            for (circle_key, circle_info) in circle_map.clone().into_iter() {
                if circle_key[..(APPROX_PRECISION - factor)]
                    == point_key[..(APPROX_PRECISION - factor)]
                {
                    if point_info.coord.vincenty_inverse(&circle_info.coord) <= radius {
                        seen_set.insert(point_key.clone());

                        circle_map
                            .entry(circle_key.clone())
                            .and_modify(|mut_circle_info| {
                                mut_circle_info.bbox.update(point_info.coord);
                                mut_circle_info.points.insert(point_key.clone());
                            });
                        point_map
                            .entry(point_key.clone())
                            .and_modify(|mut_point_info| {
                                mut_point_info.circles.insert(circle_key.clone());
                            });
                    }
                }
            }
        }
        factor += 1;
    }
    println!(
      "Post Check:\nTotal: {} | Consolidated: {} | Circle Checks: {} | Point Checks: {}\nPoints: {} / {}",
      circle_total,
      circle_map.len(),
      circle_map.values().fold(0, |acc, x| acc + x.points.len()),
      point_map.values().fold(0, |acc, x| acc + x.circles.len()),
      seen_set.len(),
      point_map.len()
  );

    let mut count = 0;
    // Cleans out empty circles, help with loop times
    let circle_map: HashMap<String, CircleInfo> = helpers::get_sorted(&circle_map)
        .into_iter()
        .filter(|(_, circle_info)| !circle_info.points.is_empty())
        .filter_map(|(circle_key, circle_info)| {
            let mut unique = HashSet::new();
            let mut points = HashSet::new();
            for point in circle_info.points.iter() {
                let point_info = point_map.get(point).unwrap();
                if point_info.circles.len() == 1 {
                    unique.insert(point.to_string());
                } else {
                    points.insert(point.to_string());
                }
            }
            if unique.is_empty() {
                count += 1;
                for point in circle_info.points.into_iter() {
                    point_map.entry(point).and_modify(|info| {
                        info.circles.remove(&circle_key);
                    });
                }
                return None;
            }
            let circle_info = CircleInfo {
                points,
                unique,
                ..circle_info
            };
            Some((circle_key, circle_info))
        })
        .collect();

    println!("Removed {}", count);
    println!(
        "Stage 1 time: {}s | Circles: {}",
        time.elapsed().as_secs_f32(),
        circle_map.len()
    );
    (point_map, circle_map)
}
