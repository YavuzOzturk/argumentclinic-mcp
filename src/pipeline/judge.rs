use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use crate::providers::Provider;

const SYSTEM: &str = r#"You are a judge in a structured reasoning pipeline. \
You receive a claim, an attack against it, and a defense. \
Determine whether the defense successfully rebutted the attack. \
Return a JSON object with exactly these fields:
{
  "survived": true/false,
  "reason": "one sentence explanation",
  "confidence": 0.0-1.0
}"#;

pub struct Judgment {
    pub survived: bool,
    pub reason: String,
    pub confidence: f64,
}

pub struct Judge {
    provider: Arc<dyn Provider>,
}

impl Judge {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn evaluate(&self, claim: &str, attack: &str, defense: &str) -> Result<Judgment> {
        let prompt = format!(
            "Claim: {claim}\n\nAttack: {attack}\n\nDefense: {defense}"
        );
        let raw = self.provider.complete(SYSTEM, &prompt).await?;
        parse_judgment(&raw)
    }
}

/// Strip markdown code fences that some models wrap JSON in.
fn strip_code_fence(s: &str) -> &str {
    let s = s.trim();
    if !s.starts_with("```") {
        return s;
    }
    let inner = s.trim_start_matches('`');
    // Skip optional language tag line (e.g. "json\n")
    let inner = inner.find('\n').map(|i| &inner[i + 1..]).unwrap_or(inner);
    inner
        .rfind("```")
        .map(|i| inner[..i].trim())
        .unwrap_or(inner.trim())
}

fn parse_judgment(raw: &str) -> Result<Judgment> {
    let json_str = strip_code_fence(raw);

    let v: serde_json::Value =
        serde_json::from_str(json_str).context("judge response is not valid JSON")?;

    let survived = v["survived"]
        .as_bool()
        .ok_or_else(|| anyhow!("judge JSON missing boolean field 'survived'"))?;

    let reason = v["reason"]
        .as_str()
        .ok_or_else(|| anyhow!("judge JSON missing string field 'reason'"))?
        .to_string();

    let confidence = v["confidence"]
        .as_f64()
        .ok_or_else(|| anyhow!("judge JSON missing numeric field 'confidence'"))?
        .clamp(0.0, 1.0);

    Ok(Judgment { survived, reason, confidence })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_json() {
        let j = parse_judgment(r#"{"survived":true,"reason":"holds up","confidence":0.8}"#)
            .unwrap();
        assert!(j.survived);
        assert_eq!(j.reason, "holds up");
        assert!((j.confidence - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_fenced_json() {
        let j = parse_judgment("```json\n{\"survived\":false,\"reason\":\"weak\",\"confidence\":0.9}\n```")
            .unwrap();
        assert!(!j.survived);
        assert!((j.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_is_clamped() {
        let j = parse_judgment(r#"{"survived":true,"reason":"fine","confidence":1.5}"#).unwrap();
        assert!((j.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_field_errors() {
        assert!(parse_judgment(r#"{"survived":true}"#).is_err());
    }
}
