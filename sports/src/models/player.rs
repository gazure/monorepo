use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Player reference data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Player {
    pub id: i32,
    pub bbref_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Player data for insertion (without id and timestamps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlayer {
    pub bbref_id: String,
    pub name: String,
}

impl NewPlayer {
    pub fn new(bbref_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            bbref_id: bbref_id.into(),
            name: name.into(),
        }
    }
}
