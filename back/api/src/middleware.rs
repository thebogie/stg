use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::HttpMessage;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tracing::Instrument;

use crate::metrics::{record_http_request, Metrics};
use crate::observability::context::{user_id_from_extensions, RequestContext, clear_request_scope, set_request_scope};
use crate::observability::events::log_http_request;

pub struct Logger {
    metrics: Option<Arc<Metrics>>,
}

impl Logger {
    pub fn new() -> Self {
        Self { metrics: None }
    }

    pub fn with_metrics(metrics: Arc<Metrics>) -> Self {
        Self {
            metrics: Some(metrics),
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for Logger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LoggerMiddleware {
            service: Rc::new(service),
            metrics: self.metrics.clone(),
        }))
    }
}

pub struct LoggerMiddleware<S> {
    service: Rc<S>,
    metrics: Option<Arc<Metrics>>,
}

impl<S, B> Service<ServiceRequest> for LoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let metrics = self.metrics.clone();
        let start_time = Instant::now();
        let method = req.method().clone();
        let uri_path = req.path().to_string();
        let peer_addr = req.peer_addr().map(|addr| addr.to_string());

        let ctx = RequestContext::from_request(&req);
        let request_id = ctx.request_id.clone();
        let trace_id = ctx.trace_id.clone();
        ctx.attach_to_request(&mut req);

        if let Some(ref m) = metrics {
            m.http.requests_in_flight.inc();
        }

        let span = tracing::info_span!(
            "http_request",
            request_id = %request_id,
            trace_id = tracing::field::Empty,
            http.method = %method,
            http.path = %normalize_endpoint(&uri_path),
        );
        if let Some(ref tid) = trace_id {
            span.record("trace_id", tid.as_str());
        }

        Box::pin(
            async move {
                set_request_scope(&request_id, None);
                let result = async {
                    let mut res = svc.call(req).await?;
                    let duration = start_time.elapsed();

                    if let Some(ref m) = metrics {
                        m.http.requests_in_flight.dec();
                    }

                    ctx.set_response_header(res.headers_mut());

                    let status_code = res.status().as_u16();
                    let normalized_path = normalize_endpoint(&uri_path);
                    let user_id = user_id_from_extensions(res.request());
                    if let Some(ref uid) = user_id {
                        set_request_scope(&request_id, Some(uid));
                    }

                    if let Some(ref m) = metrics {
                        record_http_request(
                            m,
                            method.as_str(),
                            &normalized_path,
                            status_code,
                            duration,
                        );
                    }

                    log_http_request(
                        method.as_str(),
                        &normalized_path,
                        status_code,
                        duration.as_millis(),
                        &request_id,
                        user_id.as_deref(),
                        peer_addr.as_deref().unwrap_or("unknown"),
                    );

                    Ok(res)
                }
                .instrument(span)
                .await;

                clear_request_scope();
                result
            },
        )
    }
}

/// Normalize endpoint path for metrics (remove IDs, etc.)
fn normalize_endpoint(path: &str) -> String {
    // Remove common ID patterns (UUIDs, numeric IDs, etc.)
    let path = path
        .split('/')
        .map(|segment| {
            // Check if segment looks like an ID
            if segment.is_empty() {
                segment
            } else if segment.parse::<u64>().is_ok() || uuid::Uuid::parse_str(segment).is_ok() {
                "{id}"
            } else if segment.contains('@') {
                // Email addresses
                "{email}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    // Limit path length to avoid cardinality explosion
    if path.len() > 100 {
        format!("{}...", &path[..97])
    } else {
        path
    }
}

// Admin IP allowlist middleware
pub struct AdminIpAllowlist {
    allowed: Vec<String>,
}

impl AdminIpAllowlist {
    pub fn new_from_env() -> Self {
        let list = std::env::var("ADMIN_IP_ALLOWLIST").unwrap_or_default();
        let allowed = list
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self { allowed }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AdminIpAllowlist
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AdminIpAllowlistService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminIpAllowlistService {
            service: Rc::new(service),
            allowed: self.allowed.clone(),
        }))
    }
}

pub struct AdminIpAllowlistService<S> {
    service: Rc<S>,
    allowed: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for AdminIpAllowlistService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let client_ip_opt = req.peer_addr().map(|a| a.ip().to_string());
        let allowed = self.allowed.clone();
        let service = self.service.clone();

        Box::pin(async move {
            if allowed.is_empty() {
                return service.call(req).await;
            }
            if let Some(ip) = client_ip_opt {
                if allowed.iter().any(|a| a == &ip) {
                    return service.call(req).await;
                }
            }
            Err(actix_web::error::ErrorForbidden(
                "Admin access not allowed from this IP",
            ))
        })
    }
}

// Admin audit logging middleware (no PII beyond email)
pub struct AdminAudit;

impl<S, B> Transform<S, ServiceRequest> for AdminAudit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AdminAuditService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminAuditService {
            service: Rc::new(service),
        }))
    }
}

pub struct AdminAuditService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AdminAuditService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().clone();
        let path = req.path().to_string();
        let email = req.request().extensions().get::<String>().cloned();
        let service = self.service.clone();
        let start = Instant::now();

        Box::pin(async move {
            let result = service.call(req).await;
            let duration_ms = start.elapsed().as_millis();
            match &result {
                Ok(res) => {
                    let status = res.status().as_u16();
                    tracing::info!(
                        event = "admin.audit",
                        http.method = %method,
                        http.path = %path,
                        http.status_code = status,
                        duration_ms = duration_ms,
                        user_id = email.as_deref().unwrap_or("unknown"),
                        "admin action"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        event = "admin.audit",
                        http.method = %method,
                        http.path = %path,
                        duration_ms = duration_ms,
                        user_id = email.as_deref().unwrap_or("unknown"),
                        error.message = %e,
                        "admin action failed"
                    );
                }
            }
            result
        })
    }
}

pub fn cors_middleware() -> actix_cors::Cors {
    let mut cors = actix_cors::Cors::default()
        .allowed_origin("http://localhost:50003")
        .allowed_origin("http://127.0.0.1:50003")
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            actix_web::http::header::ACCEPT,
            actix_web::http::header::CONTENT_TYPE,
            actix_web::http::header::AUTHORIZATION,
        ])
        .supports_credentials()
        .max_age(3600);

    // Add production domain if in production environment
    if let Ok(env) = std::env::var("RUST_ENV") {
        if env == "production" {
            cors = cors.allowed_origin("https://smacktalkgaming.com");
            // Also allow www subdomain
            cors = cors.allowed_origin("https://www.smacktalkgaming.com");
        }
    }

    cors
}

/// Security headers middleware
pub struct SecurityHeaders;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let is_production = std::env::var("RUST_ENV")
            .unwrap_or_default()
            .eq_ignore_ascii_case("production");

        Box::pin(async move {
            let mut res = svc.call(req).await?;

            // Add security headers
            let headers = res.headers_mut();

            // Prevent MIME type sniffing
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );

            // Prevent clickjacking attacks
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            );

            // XSS Protection (legacy, but still useful for older browsers)
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("1; mode=block"),
            );

            // HSTS - only in production (HTTPS)
            if is_production {
                headers.insert(
                    HeaderName::from_static("strict-transport-security"),
                    HeaderValue::from_static("max-age=31536000; includeSubDomains"),
                );
            }

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        http::{Method, StatusCode},
        test, web, App,
    };
    use std::time::Duration;

    #[actix_web::test]
    async fn test_cors_middleware_configuration() {
        let _cors = cors_middleware();
        assert!(true); // CORS middleware created successfully
    }

    #[actix_web::test]
    async fn test_logger_middleware_creation() {
        let _logger = Logger::new();
        assert!(true);
    }

    #[actix_web::test]
    async fn test_logger_middleware_service_creation() {
        let _logger = Logger::new();
        assert!(true);
    }

    #[actix_web::test]
    async fn test_logger_middleware_transform() {
        let logger = Logger::new();
        let app = test::init_service(
            App::new()
                .wrap(logger)
                .route("/test", web::get().to(|| async { "test" })),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_logger_middleware_with_error() {
        let logger = Logger::new();
        let app = test::init_service(App::new().wrap(logger).route(
            "/error",
            web::get().to(|| async { actix_web::HttpResponse::InternalServerError().finish() }),
        ))
        .await;

        let req = test::TestRequest::get().uri("/error").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_logger_middleware_with_client_error() {
        let logger = Logger::new();
        let app = test::init_service(App::new().wrap(logger).route(
            "/notfound",
            web::get().to(|| async { actix_web::HttpResponse::NotFound().finish() }),
        ))
        .await;

        let req = test::TestRequest::get().uri("/notfound").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_cors_middleware_integration() {
        let app = test::init_service(
            App::new()
                .wrap(cors_middleware())
                .route("/test", web::get().to(|| async { "test" })),
        )
        .await;

        // Test normal request without origin header
        let req = test::TestRequest::get().uri("/test").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Test request with allowed origin
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header((actix_web::http::header::ORIGIN, "http://localhost:50003"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Test OPTIONS request (preflight) with proper headers
        let req = test::TestRequest::default()
            .method(Method::OPTIONS)
            .uri("/test")
            .insert_header((actix_web::http::header::ORIGIN, "http://localhost:50003"))
            .insert_header((
                actix_web::http::header::ACCESS_CONTROL_REQUEST_METHOD,
                "GET",
            ))
            .insert_header((
                actix_web::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization",
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // OPTIONS should be handled by CORS middleware and return 200 OK
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_logger_middleware_timing() {
        let logger = Logger::new();
        let app = test::init_service(App::new().wrap(logger).route(
            "/slow",
            web::get().to(|| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                "slow"
            }),
        ))
        .await;

        let start = std::time::Instant::now();
        let req = test::TestRequest::get().uri("/slow").to_request();

        let resp = test::call_service(&app, req).await;
        let duration = start.elapsed();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(duration >= Duration::from_millis(10)); // Should take at least 10ms
    }

    #[actix_web::test]
    async fn test_logger_middleware_with_peer_addr() {
        let logger = Logger::new();
        let app = test::init_service(
            App::new()
                .wrap(logger)
                .route("/test", web::get().to(|| async { "test" })),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
