use anyhow::Result;

fn main() -> Result<()> {
    #[cfg(feature = "web")]
    start::start(christmas::app);

    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            christmas::server::launch(christmas::app).await;
        })
    }

    Ok(())
}
