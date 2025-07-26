use dioxus::prelude::ServerFnError;

pub type Result<T, E = ServerFnError> = core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[expect(dead_code)]
    #[error("Server Error: {0}")]
    Server(String),
    #[expect(dead_code)]
    #[error("Unknown Error {0}")]
    Unknown(String),
}
