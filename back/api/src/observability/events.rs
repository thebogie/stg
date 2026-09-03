//! Typed operational log events for startup, shutdown, DB, auth, and schedulers.

use crate::config::LoggingConfig;
use crate::observability::redact::redact_sensitive;

pub fn log_startup(config: &LoggingConfig, host: &str, port: u16) {
    tracing::info!(
        event = "service.startup",
        service = %config.service_name,
        environment = %config.environment,
        version = %config.version,
        host = %host,
        port = port,
        "service started"
    );
}

pub fn log_shutdown(reason: &str) {
    tracing::info!(
        event = "service.shutdown",
        reason = %reason,
        "service shutting down"
    );
}

pub fn log_config_error(message: &str) {
    tracing::error!(
        event = "config.error",
        error.message = %redact_sensitive(message),
        "configuration error"
    );
}

pub fn log_db_connect(url_host: &str, attempt: u32) {
    tracing::info!(
        event = "db.connect",
        db.host = %url_host,
        attempt = attempt,
        "connecting to database"
    );
}

pub fn log_db_connect_success(url_host: &str) {
    tracing::info!(
        event = "db.connect",
        db.host = %url_host,
        outcome = "success",
        "database connected"
    );
}

pub fn log_db_connect_failure(url_host: &str, attempt: u32, error: &str) {
    tracing::warn!(
        event = "db.connect",
        db.host = %url_host,
        attempt = attempt,
        outcome = "failure",
        error.message = %redact_sensitive(error),
        "database connection attempt failed"
    );
}

pub fn log_db_query_error(operation: &str, error: &str) {
    tracing::error!(
        event = "db.query.error",
        db.operation = %operation,
        error.message = %redact_sensitive(error),
        "database query failed"
    );
}

pub fn log_auth_event(event: &str, user_id: Option<&str>, detail: Option<&str>) {
    match event {
        "auth.login.success" => tracing::info!(
            event = %event,
            user_id = user_id.unwrap_or("unknown"),
            "authentication succeeded"
        ),
        "auth.login.failure" | "auth.session.invalid" => tracing::warn!(
            event = %event,
            user_id = user_id.unwrap_or("unknown"),
            detail = detail.map(redact_sensitive).as_deref(),
            "authentication failed"
        ),
        _ => tracing::info!(
            event = %event,
            user_id = user_id.unwrap_or("unknown"),
            detail = detail.map(redact_sensitive).as_deref(),
            "authentication event"
        ),
    }
}

pub fn log_scheduler_event(scheduler: &str, event: &str, detail: Option<&str>) {
    let detail_redacted = detail.map(redact_sensitive);
    if event.contains("error") || event.contains("failure") {
        tracing::error!(
            event = %event,
            scheduler = %scheduler,
            detail = detail_redacted.as_deref(),
            "scheduler event"
        );
    } else if event.contains("warn") {
        tracing::warn!(
            event = %event,
            scheduler = %scheduler,
            detail = detail_redacted.as_deref(),
            "scheduler event"
        );
    } else {
        tracing::info!(
            event = %event,
            scheduler = %scheduler,
            detail = detail_redacted.as_deref(),
            "scheduler event"
        );
    }
}

pub fn log_health_degraded(component: &str, message: &str) {
    tracing::warn!(
        event = "health.degraded",
        component = %component,
        error.message = %redact_sensitive(message),
        "health check degraded"
    );
}

pub fn log_api_error(
    error_code: &str,
    status_code: u16,
    message: &str,
    request_id: Option<&str>,
    user_id: Option<&str>,
) {
    let redacted = redact_sensitive(message);
    if status_code >= 500 {
        tracing::error!(
            event = "api.error",
            error.code = %error_code,
            http.status_code = status_code,
            error.message = %redacted,
            request_id = request_id,
            user_id = user_id,
            "API error response"
        );
    } else if status_code >= 400 {
        tracing::warn!(
            event = "api.error",
            error.code = %error_code,
            http.status_code = status_code,
            error.message = %redacted,
            request_id = request_id,
            user_id = user_id,
            "API error response"
        );
    } else {
        tracing::info!(
            event = "api.error",
            error.code = %error_code,
            http.status_code = status_code,
            error.message = %redacted,
            request_id = request_id,
            user_id = user_id,
            "API error response"
        );
    }
}

pub fn log_http_request(
    method: &str,
    path: &str,
    status_code: u16,
    duration_ms: u128,
    request_id: &str,
    user_id: Option<&str>,
    client_ip: &str,
) {
    if status_code >= 500 {
        tracing::error!(
            event = "http.request",
            http.method = %method,
            http.path = %path,
            http.status_code = status_code,
            duration_ms = duration_ms,
            request_id = %request_id,
            user_id = user_id,
            client_ip = %client_ip,
            "HTTP request completed"
        );
    } else if status_code >= 400 {
        tracing::warn!(
            event = "http.request",
            http.method = %method,
            http.path = %path,
            http.status_code = status_code,
            duration_ms = duration_ms,
            request_id = %request_id,
            user_id = user_id,
            client_ip = %client_ip,
            "HTTP request completed"
        );
    } else {
        tracing::info!(
            event = "http.request",
            http.method = %method,
            http.path = %path,
            http.status_code = status_code,
            duration_ms = duration_ms,
            request_id = %request_id,
            user_id = user_id,
            client_ip = %client_ip,
            "HTTP request completed"
        );
    }
}

/// Extract host portion from a URL for safe logging (no credentials).
pub fn url_host_for_log(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str().map(|h| {
                if let Some(port) = u.port() {
                    format!("{}:{}", h, port)
                } else {
                    h.to_string()
                }
            })
        })
        .unwrap_or_else(|| "[invalid-url]".to_string())
}
