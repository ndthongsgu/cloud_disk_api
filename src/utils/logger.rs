use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_subscriber::fmt::time::UtcTime;

pub fn init_logger() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_timer(UtcTime::rfc_3339())
                .with_thread_names(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        );

    if let Err(err) = subscriber.try_init() {
        eprintln!("Logger already initialized: {err}");
        return;
    }

    tracing::info!("Logger initialized");
}
