use tracing_subscriber::{fmt, EnvFilter};

pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("seer_api=debug,tower_http=debug,axum=debug"));

    fmt().with_env_filter(filter).compact().init();
}
