//! Structured logging

use tracing::info;

/// Logger - structured logging setup
pub struct Logger {
    level: String,
}

impl Logger {
    pub fn new(level: &str) -> Self {
        Self {
            level: level.to_string(),
        }
    }

    pub fn init(&self) {
        info!("Logger initialized with level: {}", self.level);
    }
}
