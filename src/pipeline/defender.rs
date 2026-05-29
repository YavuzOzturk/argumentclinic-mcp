use std::sync::Arc;

use anyhow::Result;

use crate::providers::Provider;

const SYSTEM: &str = "\
You are an advocate in a structured reasoning pipeline. \
Your job is to defend the original claim against the attack. \
Engage directly with the attack's specific points. \
Do not concede unless the attack identifies a fundamental flaw. \
Maximum 3 sentences.";

pub struct Defender {
    provider: Arc<dyn Provider>,
}

impl Defender {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn defend(&self, claim: &str, attack: &str) -> Result<String> {
        let prompt = format!("Claim: {claim}\n\nAttack: {attack}");
        self.provider.complete(SYSTEM, &prompt).await
    }
}
