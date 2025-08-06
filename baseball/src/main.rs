fn main() {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_max_level(if std::env::var("DEBUG").is_ok() {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    baseball::run();
}
