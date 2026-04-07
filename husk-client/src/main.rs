use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .with_timer(fmt::time::uptime())
        .init();

    info!("Husk Client is starting up...");
    debug!("PID: {}", std::process::id());

    husk_core::core_handshake_test(42);

    info!("Husk Client initialization completed.");
}
