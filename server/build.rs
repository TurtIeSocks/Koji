fn main() {
    cxx_build::bridge("src/cpp/bridge.rs")
        .file("src/cpp/clustering/main.cpp")
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-O3")
        .compile("koji");
}
