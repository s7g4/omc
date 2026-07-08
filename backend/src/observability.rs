use crate::settings::Settings;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace, trace::TracerProvider};

/// Initializes an OTLP exporter pointed at a local Jaeger instance (see `docker-compose.yml`'s
/// `jaeger` service) and returns a `tracing-opentelemetry` layer that can be added to the
/// `tracing_subscriber` registry. No-op (returns `None`) if disabled via config, so this never
/// blocks startup on a Jaeger container that isn't running.
pub fn init_tracer(
    settings: &Settings,
) -> Option<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::Tracer,
    >,
> {
    if !settings.otel.enabled {
        return None;
    }

    let exporter = match opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&settings.otel.otlp_endpoint)
        .build_span_exporter()
    {
        Ok(exporter) => exporter,
        Err(e) => {
            tracing::warn!(
                "Failed to build OTLP exporter ({}); continuing without tracing export: {:?}",
                settings.otel.otlp_endpoint,
                e
            );
            return None;
        }
    };

    let resource = opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        settings.otel.service_name.clone(),
    )]);

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(trace::Config::default().with_resource(resource))
        .build();

    let tracer = provider.tracer(settings.otel.service_name.clone());
    let _ = opentelemetry::global::set_tracer_provider(provider);

    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}
