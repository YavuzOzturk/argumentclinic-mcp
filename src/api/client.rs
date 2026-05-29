use anyhow::Result;
use crate::config::models::RoleConfig;

pub struct AgApiClient {
    api_key: String,
    http: reqwest::Client,
}

impl AgApiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }

    /// PRO mode: fetch optimal model assignments from the AG API.
    pub async fn get_model_assignments(&self) -> Result<RoleConfig> {
        todo!()
    }
}
