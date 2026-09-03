//! Structured logging, request correlation, and operational events.
//!
//! All application code should prefer `tracing` macros so fields are searchable in Loki/Grafana.
//! Legacy `log::` calls are bridged via `tracing-log`.

pub mod context;
pub mod events;
pub mod init;
pub mod redact;

pub use context::RequestContext;
pub use init::{init_observability, AppSpanGuard};
pub use redact::redact_sensitive;

/// Log at ERROR with `fatal: true` for unrecoverable process-level failures.
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)+) => {
        tracing::error!(fatal = true, $($arg)+);
    };
}
