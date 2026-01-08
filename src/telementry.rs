use opentelemetry_otlp::WithExportConfig;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

use opentelemetry::KeyValue;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::{Resource, runtime, trace as sdktrace};

use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Sync + Send + 'static,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // If no RUST_LOG is set, use the passed string (e.g., "info")
        // but explicitly quiet down the noisy crates
        EnvFilter::new(env_filter)
            .add_directive("h2=warn".parse().unwrap())
            .add_directive("hyper=warn".parse().unwrap())
            .add_directive("tonic=warn".parse().unwrap())
            .add_directive("tower=warn".parse().unwrap())
    });

    // 1. Create the OTLP Span Exporter
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()
        .expect("Failed to create OTLP exporter");

    // 2. Define your Resource
    // In v0.27+, we use Resource::new() or Resource::default().merge()
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, name.clone()),
        KeyValue::new("environment", "development"),
    ]);

    // 3. Create the Tracer Provider with the resource
    let tracer_provider = sdktrace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(resource)
        .build();

    // 4. Create the Tracer and the Tracing Layer
    let tracer = opentelemetry::trace::TracerProvider::tracer(&tracer_provider, "shorty-tracer");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // 5. Build the registry
    Registry::default()
        .with(env_filter)
        .with(telemetry_layer)
        .with(JsonStorageLayer)
        .with(BunyanFormattingLayer::new(name, sink))
}
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to initialize env log tracer");
    set_global_default(subscriber).expect("failed to create subscriber");
}
