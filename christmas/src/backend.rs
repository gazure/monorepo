use dioxus::prelude::*;

use crate::{
    Result,
    model::{Exchange, ExchangeAppConfig, ExchangePool},
};

#[server]
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
    let yaml_content = include_str!("../assets/participants.yaml");
    let config: ExchangeAppConfig =
        serde_yaml::from_str(yaml_content).map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(config.pools())
}

#[server]
pub async fn new_exchange(name: String, description: String, year: i32) -> Result<Exchange> {
    use sqlx::{Pool, Postgres, Row};
    let pool = extract::<FromContext<Pool<Postgres>>, _>()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .0;

    let row = sqlx::query(
        r#"
        INSERT INTO exchange (name, description, year, status, letters, created_at, updated_at)
        VALUES ($1, $2, $3, 'planning', 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', NOW(), NOW())
        RETURNING id, name, description, year, status, letters, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(year)
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    let name = row
        .try_get_raw(1)?
        .as_str()
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .to_string();
    let letters = row
        .try_get_raw(5)?
        .as_str()
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .to_string();

    let exchange = Exchange {
        name: name,
        letters: Some(letters),
    };

    Ok(exchange)
}

#[server]
pub async fn get_exchanges() -> Result<Vec<Exchange>> {
    use sqlx::{Pool, Postgres, Row};
    let pool = extract::<FromContext<Pool<Postgres>>, _>()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .0;

    let rows = sqlx::query(
        r#"
        SELECT id, name, description, year, status, letters, created_at, updated_at
        FROM exchange
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    let exchanges = rows
        .into_iter()
        .map(|row| {
            let name = row
                .try_get_raw(1)?
                .as_str()
                .map_err(|e| ServerFnError::ServerError(e.to_string()))?
                .to_string();
            let letters = row
                .try_get_raw(5)?
                .as_str()
                .map_err(|e| ServerFnError::ServerError(e.to_string()))?
                .to_string();

            Ok(Exchange {
                name: name,
                letters: Some(letters),
            })
        })
        .collect::<Result<Vec<Exchange>>>()?;

    Ok(exchanges)
}
