use shorty::{
    startup,
    telementry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("shorty".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    opentelemetry::global::shutdown_tracer_provider();
    startup::run().await
}
