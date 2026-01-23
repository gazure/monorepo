fn main() {
    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            christmas::init_server().await;
        });
    }

    dioxus::launch(christmas::App);
}
