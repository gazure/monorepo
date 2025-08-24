#[cfg(feature = "server")]
use dioxus::server::context::NotFoundInServerContext;
use serde::Serialize;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Server(String),
    #[error("Serde Error: {0}")]
    Serde(String),
    #[error("Database Error: {0}")]
    Database(String),
    #[expect(dead_code)]
    #[error("Unknown Error {0}")]
    Unknown(String),
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::Serde(err.to_string())
    }
}

#[cfg(feature = "server")]
impl<T: 'static> From<NotFoundInServerContext<T>> for Error {
    fn from(value: NotFoundInServerContext<T>) -> Self {
        Self::Server(value.to_string())
    }
}

#[cfg(feature = "server")]
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value.to_string())
    }
}

#[cfg(feature = "server")]
impl From<sqlx::migrate::MigrateError> for Error {
    fn from(value: sqlx::migrate::MigrateError) -> Self {
        Self::Database(value.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
