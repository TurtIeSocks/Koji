#[cxx::bridge]
mod ffi {
    struct CppPoint {
        x: f64,
        y: f64,
    }

    unsafe extern "C++" {
        include!("koji/src/clustering/main.h");

        fn concat(elements: Vec<CppPoint>) -> Vec<CppPoint>;
    }
}

pub fn cpp_cluster(points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let shared = |x, y| ffi::CppPoint { x, y };
    let elements = points.iter().map(|point| shared(point[0], point[1])).collect();

    let output_points = ffi::concat(elements);
    println!("Rust Print:");
    let mut points = Vec::<[f64; 2]>::new();
    for coord in output_points {
        // println!("{}", coord);
        points.push([coord.x, coord.y]);
    }
    points
}
