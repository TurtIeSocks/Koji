#[cfg(target_os = "linux")]
#[global_allocator]
static A: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    koji_service::init_env_and_logging();

    if let Err(err) = koji_service::start() {
        log::error!(
            "[KOJI] Kōji encountered a critical error and shut down: {:?}",
            err
        )
    }
}
