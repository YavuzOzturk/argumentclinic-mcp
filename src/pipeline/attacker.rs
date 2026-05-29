use std::sync::Arc;

use anyhow::Result;

use crate::providers::Provider;

const SYSTEM: &str = "\
You are an adversarial critic in a structured reasoning pipeline. \
Your job is to find the strongest possible objection to the claim you receive. \
Be specific, cite what's missing or wrong, and propose what a better version of \
the claim would require. Maximum 3 sentences.";

pub struct Attacker {
    provider: Arc<dyn Provider>,
}

impl Attacker {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn attack(&self, claim: &str) -> Result<String> {
        self.provider.complete(SYSTEM, claim).await
    }
}
