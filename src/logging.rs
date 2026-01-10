use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Registry,
};
use tracing_error::ErrorLayer;
use std::env;

/// Initializes the logging and tracing system.
/// 
/// Supports two modes based on the `APP_ENV` environment variable:
/// - `development` (default): Pretty-printed, colored logs for console.
/// - `production`: JSON-formatted logs for aggregation (Datadog, ELK, etc.).
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("clear_urls_bot=info,teloxide=info,axum=info"));

    let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

    let registry = Registry::default()
        .with(env_filter)
        .with(ErrorLayer::default());

    if env == "production" {
        let json_layer = fmt::layer()
            .json()
            .with_thread_ids(true)
            .with_target(true);
        
        registry.with(json_layer).init();
    } else {
        let fmt_layer = fmt::layer()
            .pretty()
            .with_thread_ids(true)
            .with_target(true);
            
        registry.with(fmt_layer).init();
    }

    tracing::info!(env = %env, "Logging system initialized");
}

/// Debugging utility for tracking execution time of a block/future.
pub struct Timer {
    label: &'static str,
    start: std::time::Instant,
}

impl Timer {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            start: std::time::Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        tracing::debug!(
            label = %self.label,
            duration_ms = %duration.as_millis(),
            "Operation completed"
        );
    }
}
