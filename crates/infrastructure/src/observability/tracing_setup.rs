//! Tracing setup

use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .init();
}
