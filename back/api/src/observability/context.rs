//! Per-request correlation context propagated through Actix extensions and tracing spans.

use actix_web::HttpMessage;
use actix_web::{
    dev::ServiceRequest,
    http::header::{HeaderName, HeaderValue},
};
use std::cell::RefCell;
use std::fmt;
use tracing::Span;
use uuid::Uuid;

thread_local! {
    static REQUEST_SCOPE: RefCell<Option<(String, Option<String>)>> = const { RefCell::new(None) };
}

/// Set per-request scope for code that cannot access Actix extensions (e.g. `ApiError`).
pub fn set_request_scope(request_id: &str, user_id: Option<&str>) {
    REQUEST_SCOPE.with(|cell| {
        *cell.borrow_mut() = Some((
            request_id.to_string(),
            user_id.map(str::to_string),
        ));
    });
}

/// Clear the per-request scope at the end of an HTTP request.
pub fn clear_request_scope() {
    REQUEST_SCOPE.with(|cell| *cell.borrow_mut() = None);
}

/// Current request ID when handling an HTTP request.
pub fn current_request_id() -> Option<String> {
    REQUEST_SCOPE.with(|cell| cell.borrow().as_ref().map(|(id, _)| id.clone()))
}

/// Current authenticated user ID (email) when available.
pub fn current_user_id() -> Option<String> {
    REQUEST_SCOPE.with(|cell| cell.borrow().as_ref().and_then(|(_, u)| u.clone()))
}

/// Correlation identifiers for a single HTTP request.
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: Option<String>,
}

impl RequestContext {
    pub fn from_request(req: &ServiceRequest) -> Self {
        let request_id = resolve_request_id(req);
        let trace_id = parse_traceparent(req);
        Self {
            request_id,
            trace_id,
        }
    }

    pub fn attach_to_request(&self, req: &mut ServiceRequest) {
        req.extensions_mut().insert(self.clone());
    }

    pub fn attach_to_span(&self) {
        let span = Span::current();
        span.record("request_id", &self.request_id.as_str());
        if let Some(ref trace_id) = self.trace_id {
            span.record("trace_id", trace_id.as_str());
        }
    }

    pub fn set_response_header(&self, headers: &mut actix_web::http::header::HeaderMap) {
        if let Ok(value) = HeaderValue::try_from(self.request_id.as_str()) {
            headers.insert(HeaderName::from_static("x-request-id"), value);
        }
    }
}

impl fmt::Display for RequestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.request_id)
    }
}

/// Read the authenticated user email from request extensions (set by auth middleware).
pub fn user_id_from_extensions<T: HttpMessage>(message: &T) -> Option<String> {
    message.extensions().get::<String>().cloned()
}

fn resolve_request_id(req: &ServiceRequest) -> String {
    if let Some(id) = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
    {
        return id.to_string();
    }
    generate_request_id()
}

/// Parse W3C `traceparent` header: `00-{trace-id}-{span-id}-{flags}`.
fn parse_traceparent(req: &ServiceRequest) -> Option<String> {
    let value = req.headers().get("traceparent")?.to_str().ok()?;
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() >= 4 && parts[0] == "00" && parts[1].len() == 32 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Generate a request ID — fast counter for tests, UUID v4 otherwise.
pub fn generate_request_id() -> String {
    let is_test = cfg!(test)
        || std::env::var("RUST_ENV")
            .unwrap_or_default()
            .eq_ignore_ascii_case("test");

    if is_test {
        use std::sync::atomic::{AtomicU64, Ordering};
        static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("test-{}", counter)
    } else {
        Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn honors_incoming_request_id() {
        let id = Uuid::new_v4().to_string();
        let req = TestRequest::default()
            .insert_header(("x-request-id", id.as_str()))
            .to_srv_request();
        let ctx = RequestContext::from_request(&req);
        assert_eq!(ctx.request_id, id);
    }

    #[test]
    fn parses_traceparent() {
        let req = TestRequest::default()
            .insert_header((
                "traceparent",
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            ))
            .to_srv_request();
        let ctx = RequestContext::from_request(&req);
        assert_eq!(
            ctx.trace_id.as_deref(),
            Some("4bf92f3577b34da6a3ce929d0e0e4736")
        );
    }
}
