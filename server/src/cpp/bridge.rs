#[cxx::bridge]
pub mod ffi {
    struct CppPoint {
        x: f64,
        y: f64,
    }

    unsafe extern "C++" {
        include!("koji/src/cpp/clustering/main.h");

        fn clustering(elements: Vec<CppPoint>, fast: u8) -> Vec<CppPoint>;
    }
}
