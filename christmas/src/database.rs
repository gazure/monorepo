use sqlx::PgPool;

/// Failure modes of first-use database initialization.
#[derive(Debug)]
pub enum InitError {
    Connect(sqlx::Error),
    Migrate(sqlx::migrate::MigrateError),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(e) => write!(f, "connect: {e}"),
            Self::Migrate(e) => write!(f, "migrate: {e}"),
        }
    }
}

impl std::error::Error for InitError {}

impl From<sqlx::Error> for InitError {
    fn from(e: sqlx::Error) -> Self {
        Self::Connect(e)
    }
}

impl From<sqlx::migrate::MigrateError> for InitError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::Migrate(e)
    }
}

pub async fn initialize(pool: &PgPool) -> Result<(), InitError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
