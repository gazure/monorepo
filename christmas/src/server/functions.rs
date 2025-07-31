use dioxus::prelude::*;

use crate::model::{Exchange, ExchangePool};

#[server]
pub async fn load_exchanges() -> ServerFnResult<Vec<ExchangePool>> {
    impls::load_exchanges().await.map_err(|e| e.into())
}

#[server]
pub async fn new_exchange(name: String, description: String, year: i32) -> ServerFnResult<Exchange> {
    impls::new_exchange(name, description, year).await.map_err(|e| e.into())
}

#[server]
pub async fn get_exchanges() -> ServerFnResult<Vec<Exchange>> {
    impls::get_exchanges().await.map_err(|e| e.into())
}

#[cfg(feature = "server")]
mod impls {
    use dioxus::prelude::*;
    use sqlx::{Pool, Postgres};

    use crate::{
        Result,
        model::{Exchange, ExchangeAppConfig, ExchangePool},
    };
    pub async fn load_exchanges() -> Result<Vec<ExchangePool>> {
        // Now you can use the pool for database operations
        // Example: Load exchanges from database
        // let exchanges = sqlx::query_as::<_, ExchangePool>(
        //     "SELECT id, name, participants FROM exchange_pools"
        // )
        // .fetch_all(&pool)
        // .await
        // .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

        // For now, keeping the YAML loading but you can replace this with database queries
        let yaml_content = include_str!("../../assets/participants.yaml");
        let config: ExchangeAppConfig = serde_yaml::from_str(yaml_content)?;
        Ok(config.pools())
    }

    pub async fn new_exchange(name: String, description: String, year: i32) -> Result<Exchange> {
        let pool = extract::<FromContext<Pool<Postgres>>, _>().await?.0;

        let row = sqlx::query!(
            r#"
            INSERT INTO exchange (name, description, year, status, letters, created_at, updated_at)
            VALUES ($1, $2, $3, 'planning', 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', NOW(), NOW())
            RETURNING id, name, description, year, status, letters, created_at, updated_at
            "#,
            name,
            description,
            year
        )
        .fetch_one(&pool)
        .await?;

        Ok(Exchange {
            name: row.name,
            letters: row.letters,
        })
    }

    pub async fn get_exchanges() -> Result<Vec<Exchange>> {
        let pool = extract::<FromContext<Pool<Postgres>>, _>().await?.0;

        let rows = sqlx::query!(
            r#"
            SELECT id, name, description, year, status, letters, created_at, updated_at
            FROM exchange
            "#,
        )
        .fetch_all(&pool)
        .await?;

        let exchanges = rows
            .into_iter()
            .map(|row| {
                Ok(Exchange {
                    name: row.name,
                    letters: row.letters,
                })
            })
            .collect::<Result<Vec<Exchange>>>()?;
        Ok(exchanges)
    }
}
