use std::thread;

use time::Duration;
use travelling_salesman::Tour;

use model::api::{multi_vec::MultiVec, single_vec::SingleVec};

pub fn base(segment: SingleVec, routing_time: i64, fast: bool) -> Tour {
    travelling_salesman::simulated_annealing::solve(
        &segment
            .iter()
            .map(|[x, y]| (*x, *y))
            .collect::<Vec<(f64, f64)>>()[0..segment.len()],
        Duration::seconds(if routing_time > 0 {
            routing_time
        } else {
            ((segment.len() as f32 / 100.) + 1.)
                .powf(if fast { 1. } else { 1.25 })
                .floor() as i64
        }),
    )
}

pub fn multi(
    clusters: &SingleVec,
    route_chunk_size: usize,
    routing_time: i64,
    fast: bool,
) -> Vec<usize> {
    let split_routes: MultiVec = if route_chunk_size > 0 {
        clusters
            .chunks(route_chunk_size)
            .map(|s| s.into())
            .collect()
    } else {
        vec![clusters.clone()]
    };

    let mut merged_routes = vec![];

    for (i, segment) in split_routes.into_iter().enumerate() {
        println!("Creating thread: {}", i + 1);
        merged_routes.push(thread::spawn(move || -> Tour {
            base(segment, routing_time, fast)
        }));
    }

    merged_routes
        .into_iter()
        .enumerate()
        .flat_map(|(i, c)| {
            c.join()
                .unwrap()
                .route
                .into_iter()
                .map(|p| p + (i * route_chunk_size))
                .collect::<Vec<usize>>()
        })
        .collect()
}
