use {
    tracing_appender::non_blocking::WorkerGuard,
    tracing_subscriber::{
        EnvFilter,
        Layer as _,
        layer::SubscriberExt as _,
        util::SubscriberInitExt as _,
    },
};

pub fn init() -> WorkerGuard {
    let filter = EnvFilter::try_from_default_env()
        .ok()
        .unwrap_or_else(|| EnvFilter::new(tracing::Level::INFO.to_string()));

    let layer = tracing_subscriber::fmt::layer()
        .event_format(tracing_subscriber::fmt::format())
        .with_target(true)
        .with_ansi(true);

    let (writer, guard) = tracing_appender::non_blocking(std::io::stderr());

    tracing_subscriber::registry()
        .with(layer.with_writer(writer).with_filter(filter))
        .init();

    guard
}
