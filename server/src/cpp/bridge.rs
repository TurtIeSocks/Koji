#[cxx::bridge]
mod ffi {
    struct CppPoint {
        x: f64,
        y: f64,
    }

    unsafe extern "C++" {
        include!("koji/src/cpp/clustering/main.h");

        fn clustering(elements: Vec<CppPoint>) -> Vec<CppPoint>;
    }
}

pub fn cpp_cluster(points: Vec<[f64; 2]>, radius: f64) -> Vec<[f64; 2]> {
    let shared = |x, y| ffi::CppPoint {
        x: x * radius,
        y: y * radius,
    };
    let elements = points
        .iter()
        .map(|point| shared(point[1], point[0]))
        .collect();

    let output_points = ffi::clustering(elements);
    let mut points = Vec::<[f64; 2]>::new();
    println!("Clusters: {}", output_points.len());
    for coord in output_points {
        points.push([coord.y / radius, coord.x / radius]);
    }
    points
}
