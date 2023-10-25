#[cfg(target_env = "gnu")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    dotenv::from_filename(std::env::var("ENV").unwrap_or(".env".to_string())).ok();

    // error | warn | info | debug | trace
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::new()
            .default_filter_or(std::env::var("LOG_LEVEL").unwrap_or("info".to_string())),
    );
    builder.target(env_logger::Target::Stdout);

    builder.init();

    if let Err(err) = api::start() {
        log::error!(
            "[KOJI] Kōji encountered a critical error and shut down: {:?}",
            err
        )
    }
}
