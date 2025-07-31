use anyhow::Result;

fn main() -> Result<()> {
    #[cfg(feature = "web")]
    start::start(christmas::app);

    #[cfg(feature = "server")]
    {
        let use_embedded = std::env::args().any(|arg| arg == "--embedded");
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            christmas::server::launch(christmas::app, use_embedded).await;
        })
    }

    Ok(())
}
