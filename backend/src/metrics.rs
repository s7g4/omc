use axum::{
    body::Body,
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{Counter, CounterVec, Encoder, Gauge, HistogramVec, Registry, TextEncoder};
use std::sync::OnceLock;
use std::time::Instant;

pub static REGISTRY: OnceLock<Registry> = OnceLock::new();
pub static HTTP_REQUESTS_TOTAL: OnceLock<CounterVec> = OnceLock::new();
pub static HTTP_REQUEST_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();
pub static TELEMETRY_INGESTED_TOTAL: OnceLock<Counter> = OnceLock::new();
pub static ACTIVE_WEBSOCKET_CONNECTIONS: OnceLock<Gauge> = OnceLock::new();

pub fn init_metrics() {
    let registry = Registry::new();

    // Register process metrics (CPU, memory, file descriptors etc)
    #[cfg(target_os = "linux")]
    {
        let process_collector = prometheus::process_collector::ProcessCollector::for_self();
        let _ = registry.register(Box::new(process_collector));
    }

    let http_requests = CounterVec::new(
        prometheus::Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status"],
    )
    .unwrap();
    registry.register(Box::new(http_requests.clone())).unwrap();

    let http_duration = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        ),
        &["method", "path"],
    )
    .unwrap();
    registry.register(Box::new(http_duration.clone())).unwrap();

    let telemetry_ingested = Counter::new(
        "telemetry_ingested_total",
        "Total number of telemetry records ingested",
    )
    .unwrap();
    registry
        .register(Box::new(telemetry_ingested.clone()))
        .unwrap();

    let active_ws = Gauge::new(
        "active_websocket_connections",
        "Current number of active WebSocket connections",
    )
    .unwrap();
    registry.register(Box::new(active_ws.clone())).unwrap();

    let _ = REGISTRY.set(registry);
    let _ = HTTP_REQUESTS_TOTAL.set(http_requests);
    let _ = HTTP_REQUEST_DURATION_SECONDS.set(http_duration);
    let _ = TELEMETRY_INGESTED_TOTAL.set(telemetry_ingested);
    let _ = ACTIVE_WEBSOCKET_CONNECTIONS.set(active_ws);
}

pub async fn track_metrics(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();

    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    let latency = start.elapsed().as_secs_f64();

    if let Some(counter) = HTTP_REQUESTS_TOTAL.get() {
        counter.with_label_values(&[&method, &path, &status]).inc();
    }

    if let Some(hist) = HTTP_REQUEST_DURATION_SECONDS.get() {
        hist.with_label_values(&[&method, &path]).observe(latency);
    }

    response
}

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];

    let metric_families = if let Some(registry) = REGISTRY.get() {
        registry.gather()
    } else {
        prometheus::gather()
    };

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode prometheus metrics: {:?}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to gather metrics",
        )
            .into_response();
    }

    Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
        .into_response()
}
