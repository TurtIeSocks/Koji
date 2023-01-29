fn main() {
    // error | warn | info | debug | trace
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(std::env::var("LOG_LEVEL").unwrap_or("info".to_string())),
    );

    if let Err(err) = api::start() {
        log::error!(
            "[KOJI] Kōji encountered a critical error and shut down: {:?}",
            err
        )
    }
}
