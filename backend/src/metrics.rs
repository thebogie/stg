use prometheus::{
    register_histogram_vec, register_int_counter_vec, HistogramOpts, HistogramVec, IntCounterVec,
    IntGauge, Opts, Registry,
};
use std::sync::Arc;
use std::time::Duration;

/// Global metrics registry
static REGISTRY: once_cell::sync::Lazy<Arc<Registry>> =
    once_cell::sync::Lazy::new(|| Arc::new(Registry::new()));

/// Global metrics instance (set after initialization)
static METRICS: std::sync::Mutex<Option<Arc<Metrics>>> = std::sync::Mutex::new(None);

/// HTTP request metrics
pub struct HttpMetrics {
    /// Request duration histogram (in seconds)
    pub request_duration: HistogramVec,
    /// Total HTTP requests counter
    pub requests_total: IntCounterVec,
    /// Active requests gauge
    pub requests_in_flight: IntGauge,
}

/// Database metrics
pub struct DatabaseMetrics {
    /// Database query duration histogram (in seconds)
    pub query_duration: HistogramVec,
    /// Total database queries counter
    pub queries_total: IntCounterVec,
    /// Database connection pool size gauge
    pub connection_pool_size: IntGauge,
    /// Active database connections gauge
    pub active_connections: IntGauge,
}

/// Redis metrics
pub struct RedisMetrics {
    /// Redis operation duration histogram (in seconds)
    pub operation_duration: HistogramVec,
    /// Total Redis operations counter
    pub operations_total: IntCounterVec,
    /// Redis connection pool size gauge
    pub connection_pool_size: IntGauge,
}

/// Scheduler metrics
pub struct SchedulerMetrics {
    /// Scheduler execution duration histogram (in seconds)
    pub execution_duration: HistogramVec,
    /// Total scheduler executions counter
    pub executions_total: IntCounterVec,
    /// Scheduler status gauge (1 = running, 0 = stopped)
    pub scheduler_status: IntGauge,
}

/// All application metrics
pub struct Metrics {
    pub http: HttpMetrics,
    pub database: DatabaseMetrics,
    pub redis: RedisMetrics,
    pub scheduler: SchedulerMetrics,
}

impl Metrics {
    /// Initialize all metrics and register them with Prometheus
    pub fn new() -> Result<Self, prometheus::Error> {
        // HTTP metrics
        let request_duration = register_histogram_vec!(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            )
            .namespace("stg")
            .subsystem("http"),
            &["method", "endpoint", "status_code"]
        )?;

        let requests_total = register_int_counter_vec!(
            Opts::new("http_requests_total", "Total number of HTTP requests")
                .namespace("stg")
                .subsystem("http"),
            &["method", "endpoint", "status_code"]
        )?;

        let requests_in_flight = IntGauge::with_opts(
            Opts::new(
                "http_requests_in_flight",
                "Number of HTTP requests currently being processed",
            )
            .namespace("stg")
            .subsystem("http"),
        )?;
        REGISTRY.register(Box::new(requests_in_flight.clone()))?;

        // Database metrics
        let query_duration = register_histogram_vec!(
            HistogramOpts::new(
                "database_query_duration_seconds",
                "Database query duration in seconds"
            )
            .namespace("stg")
            .subsystem("database"),
            &["operation", "collection"]
        )?;

        let queries_total = register_int_counter_vec!(
            Opts::new("database_queries_total", "Total number of database queries")
                .namespace("stg")
                .subsystem("database"),
            &["operation", "collection", "status"]
        )?;

        let connection_pool_size = IntGauge::with_opts(
            Opts::new(
                "database_connection_pool_size",
                "Database connection pool size",
            )
            .namespace("stg")
            .subsystem("database"),
        )?;
        REGISTRY.register(Box::new(connection_pool_size.clone()))?;

        let active_connections = IntGauge::with_opts(
            Opts::new(
                "database_active_connections",
                "Number of active database connections",
            )
            .namespace("stg")
            .subsystem("database"),
        )?;
        REGISTRY.register(Box::new(active_connections.clone()))?;

        // Redis metrics
        let operation_duration = register_histogram_vec!(
            HistogramOpts::new(
                "redis_operation_duration_seconds",
                "Redis operation duration in seconds"
            )
            .namespace("stg")
            .subsystem("redis"),
            &["operation"]
        )?;

        let operations_total = register_int_counter_vec!(
            Opts::new("redis_operations_total", "Total number of Redis operations")
                .namespace("stg")
                .subsystem("redis"),
            &["operation", "status"]
        )?;

        let redis_connection_pool_size = IntGauge::with_opts(
            Opts::new("redis_connection_pool_size", "Redis connection pool size")
                .namespace("stg")
                .subsystem("redis"),
        )?;
        REGISTRY.register(Box::new(redis_connection_pool_size.clone()))?;

        // Scheduler metrics
        let execution_duration = register_histogram_vec!(
            HistogramOpts::new(
                "scheduler_execution_duration_seconds",
                "Scheduler execution duration in seconds"
            )
            .namespace("stg")
            .subsystem("scheduler"),
            &["job_type"]
        )?;

        let executions_total = register_int_counter_vec!(
            Opts::new(
                "scheduler_executions_total",
                "Total number of scheduler executions"
            )
            .namespace("stg")
            .subsystem("scheduler"),
            &["job_type", "status"]
        )?;

        let scheduler_status = IntGauge::with_opts(
            Opts::new(
                "scheduler_status",
                "Scheduler status (1 = running, 0 = stopped)",
            )
            .namespace("stg")
            .subsystem("scheduler"),
        )?;
        REGISTRY.register(Box::new(scheduler_status.clone()))?;

        Ok(Metrics {
            http: HttpMetrics {
                request_duration,
                requests_total,
                requests_in_flight,
            },
            database: DatabaseMetrics {
                query_duration,
                queries_total,
                connection_pool_size,
                active_connections,
            },
            redis: RedisMetrics {
                operation_duration,
                operations_total,
                connection_pool_size: redis_connection_pool_size,
            },
            scheduler: SchedulerMetrics {
                execution_duration,
                executions_total,
                scheduler_status,
            },
        })
    }

    /// Get the Prometheus registry
    pub fn registry() -> Arc<Registry> {
        REGISTRY.clone()
    }

    /// Get the global metrics instance (if initialized)
    pub fn global() -> Option<Arc<Metrics>> {
        // Use try_lock to avoid blocking in tests if mutex is held
        // If lock fails, just return None (metrics not available)
        METRICS.try_lock().ok().and_then(|m| m.clone())
    }

    /// Set the global metrics instance (should be called once at startup)
    pub fn set_global(metrics: Arc<Metrics>) {
        *METRICS.lock().unwrap() = Some(metrics);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new().expect("Failed to initialize metrics")
    }
}

/// Helper function to record HTTP request metrics
pub fn record_http_request(
    metrics: &Metrics,
    method: &str,
    endpoint: &str,
    status_code: u16,
    duration: Duration,
) {
    let duration_seconds = duration.as_secs_f64();
    let status_str = status_code.to_string();

    metrics
        .http
        .request_duration
        .with_label_values(&[method, endpoint, &status_str])
        .observe(duration_seconds);

    metrics
        .http
        .requests_total
        .with_label_values(&[method, endpoint, &status_str])
        .inc();
}

/// Helper function to record database query metrics
pub fn record_database_query(
    metrics: &Metrics,
    operation: &str,
    collection: &str,
    status: &str,
    duration: Duration,
) {
    let duration_seconds = duration.as_secs_f64();

    metrics
        .database
        .query_duration
        .with_label_values(&[operation, collection])
        .observe(duration_seconds);

    metrics
        .database
        .queries_total
        .with_label_values(&[operation, collection, status])
        .inc();
}

/// Helper function to record Redis operation metrics
pub fn record_redis_operation(
    metrics: &Metrics,
    operation: &str,
    status: &str,
    duration: Duration,
) {
    let duration_seconds = duration.as_secs_f64();

    metrics
        .redis
        .operation_duration
        .with_label_values(&[operation])
        .observe(duration_seconds);

    metrics
        .redis
        .operations_total
        .with_label_values(&[operation, status])
        .inc();
}

/// Helper function to record scheduler execution metrics
pub fn record_scheduler_execution(
    metrics: &Metrics,
    job_type: &str,
    status: &str,
    duration: Duration,
) {
    let duration_seconds = duration.as_secs_f64();

    metrics
        .scheduler
        .execution_duration
        .with_label_values(&[job_type])
        .observe(duration_seconds);

    metrics
        .scheduler
        .executions_total
        .with_label_values(&[job_type, status])
        .inc();
}
