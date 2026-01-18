use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Team reference data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// Team data for insertion (without id and timestamps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTeam {
    pub code: String,
    pub name: String,
}

impl NewTeam {
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
        }
    }
}
