use dioxus::prelude::*;

use crate::model::{ExchangeAppConfig, ExchangePool};
use crate::Result;

#[server]
pub async fn load_exchanges() -> Result<Vec<ExchangePool>> {
    let yaml_content = include_str!("../assets/participants.yaml");
    let config: ExchangeAppConfig = serde_yaml::from_str(yaml_content)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(config.pools())
}
