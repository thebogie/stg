//! Initialize tracing subscriber with JSON (production) or pretty (development) output.

use crate::config::LoggingConfig;
use std::io;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Keeps the application root span entered for the process lifetime.
pub struct AppSpanGuard {
    _span: tracing::span::EnteredSpan,
}

/// Initialize structured logging. Call once at process startup before other log output.
pub fn init_observability(config: &LoggingConfig) -> Result<AppSpanGuard, String> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.default_filter));

    let use_json = config.format.eq_ignore_ascii_case("json");

    // tracing-subscriber `try_init()` installs LogTracer when the `tracing-log` feature is on.
    if use_json {
        Registry::default()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_writer(io::stdout)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false),
            )
            .try_init()
            .map_err(|e| format!("failed to initialize tracing subscriber: {}", e))?;
    } else {
        Registry::default()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(io::stdout)
                    .with_ansi(true)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false),
            )
            .try_init()
            .map_err(|e| format!("failed to initialize tracing subscriber: {}", e))?;
    }

    let span = tracing::info_span!(
        "application",
        service = %config.service_name,
        environment = %config.environment,
        version = %config.version,
    );

    Ok(AppSpanGuard {
        _span: span.entered(),
    })
}
