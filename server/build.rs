fn main() {
    cxx_build::bridge("src/clustering/bridge.rs")
        .file("src/clustering/main.cpp")
        .flag_if_supported("-std=c++14")
        .compile("koji");
}
