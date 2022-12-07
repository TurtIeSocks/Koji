fn main() {
    if let Err(err) = app::main() {
        println!("Error: {}", err);
    }
}
