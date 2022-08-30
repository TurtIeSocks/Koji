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
    let points = points
        .iter()
        .map(|point| ffi::CppPoint {
            x: point[1] * radius,
            y: point[0] * radius,
        })
        .collect();
    let points = ffi::clustering(points);
    let points: Vec<[f64; 2]> = points
        .iter()
        .map(|point| [point.y / radius, point.x / radius])
        .collect();
    points
}
