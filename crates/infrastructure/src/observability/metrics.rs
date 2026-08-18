//! Metrics collection

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics collector
#[derive(Clone)]
pub struct Metrics {
    counters: Arc<HashMap<String, AtomicU64>>,
}

impl Metrics {
    pub fn new() -> Self {
        let mut counters = HashMap::new();
        counters.insert("requests_total".to_string(), AtomicU64::new(0));
        counters.insert("errors_total".to_string(), AtomicU64::new(0));
        counters.insert("llm_calls_total".to_string(), AtomicU64::new(0));
        Self {
            counters: Arc::new(counters),
        }
    }

    pub fn increment(&self, name: &str) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn get(&self, name: &str) -> u64 {
        self.counters
            .get(name)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }
}
