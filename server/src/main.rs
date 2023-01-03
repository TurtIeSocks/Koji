fn main() {
    if let Err(err) = api::main() {
        println!("Error: {}", err);
    }
}
