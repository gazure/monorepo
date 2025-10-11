use dioxus::prelude::*;

use crate::model::{Exchange, ExchangePool};

#[server]
pub async fn load_exchanges() -> ServerFnResult<Vec<ExchangePool>> {
    impls::load_exchanges()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn new_exchange(name: String, description: String, year: i32) -> ServerFnResult<Exchange> {
    impls::new_exchange(name, description, year)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_exchanges() -> ServerFnResult<Vec<Exchange>> {
    impls::get_exchanges()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[cfg(feature = "server")]
mod impls {
    use crate::{
        Result,
        model::{Exchange, ExchangeAppConfig, ExchangePool},
    };

    pub async fn load_exchanges() -> Result<Vec<ExchangePool>> {
        // For now, keeping the YAML loading but you can replace this with database queries
        let yaml_content = include_str!("../../assets/participants.yaml");
        let config: ExchangeAppConfig = serde_yaml::from_str(yaml_content)?;
        Ok(config.pools())
    }

    pub async fn new_exchange(name: String, _description: String, _year: i32) -> Result<Exchange> {
        // For now, return a mock exchange until database pool extraction is configured
        // In production, you would get the pool from the axum state that was configured in launch.rs
        Ok(Exchange {
            name,
            letters: Some("ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string()),
        })

        // When database is properly configured, use this code:
        // The pool would be injected via the axum State extractor
        // use sqlx::{Pool, Postgres};
        // let pool: Pool<Postgres> = todo!("Get pool from server context");
        //
        // let row = sqlx::query!(
        // r#"
        // INSERT INTO exchange (name, description, year, status, letters, created_at, updated_at)
        // VALUES ($1, $2, $3, 'planning', 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', NOW(), NOW())
        // RETURNING id, name, description, year, status, letters, created_at, updated_at
        // "#,
        // name,
        // description,
        // year
        // )
        // .fetch_one(&pool)
        // .await?;
        //
        // Ok(Exchange {
        // name: row.name,
        // letters: row.letters,
        // })
    }

    pub async fn get_exchanges() -> Result<Vec<Exchange>> {
        // For now, return an empty list until database pool extraction is configured
        Ok(vec![])

        // When database is properly configured, use this code:
        // The pool would be injected via the axum State extractor
        // use sqlx::{Pool, Postgres};
        // let pool: Pool<Postgres> = todo!("Get pool from server context");
        //
        // let rows = sqlx::query!(
        // r#"
        // SELECT id, name, description, year, status, letters, created_at, updated_at
        // FROM exchange
        // "#,
        // )
        // .fetch_all(&pool)
        // .await?;
        //
        // let exchanges = rows
        // .into_iter()
        // .map(|row| {
        // Ok(Exchange {
        // name: row.name,
        // letters: row.letters,
        // })
        // })
        // .collect::<Result<Vec<Exchange>>>()?;
        // Ok(exchanges)
    }
}
