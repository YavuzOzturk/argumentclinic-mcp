use std::sync::Arc;

use anyhow::Result;

use crate::providers::Provider;
use super::{Verdict, judge::Judgment};

const SYSTEM: &str = "\
You are a synthesis engine in a structured reasoning pipeline. \
You receive a claim that has been through adversarial debate. \
If the judgment shows the claim did not survive with high confidence, \
respond with exactly: PAUSED: followed by the reason. \
Otherwise produce a refined version of the claim that incorporates \
what survived the debate. Be concise.";

const PAUSED_THRESHOLD: f64 = 0.7;

pub struct SynthesisResult {
    pub verdict: Verdict,
    pub reasoning: String,
}

pub struct Synthesizer {
    provider: Arc<dyn Provider>,
}

impl Synthesizer {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn synthesize(
        &self,
        claim: &str,
        attack: &str,
        defense: &str,
        judgment: &Judgment,
    ) -> Result<SynthesisResult> {
        let prompt = format!(
            "Claim: {claim}\n\nAttack: {attack}\n\nDefense: {defense}\n\nJudgment:\n- Survived: {}\n- Confidence: {:.2}\n- Reason: {}",
            if judgment.survived { "yes" } else { "no" },
            judgment.confidence,
            judgment.reason,
        );

        let raw = self.provider.complete(SYSTEM, &prompt).await?;
        Ok(parse_synthesis(raw, judgment))
    }
}

fn parse_synthesis(raw: String, judgment: &Judgment) -> SynthesisResult {
    // Honour the PAUSED signal either from the LLM's response or from the
    // judgment values directly (guards against models that don't follow instructions).
    let paused_by_judgment = !judgment.survived && judgment.confidence > PAUSED_THRESHOLD;
    let paused_by_response = raw.trim_start().starts_with("PAUSED:");

    if paused_by_judgment || paused_by_response {
        let reason = if paused_by_response {
            raw.trim_start()
                .strip_prefix("PAUSED:")
                .unwrap_or("")
                .trim()
                .to_string()
        } else {
            judgment.reason.clone()
        };
        return SynthesisResult {
            verdict: Verdict::Paused,
            reasoning: reason,
        };
    }

    SynthesisResult {
        verdict: Verdict::Supported,
        reasoning: raw.trim().to_string(),
    }
}
