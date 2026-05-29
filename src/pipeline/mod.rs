pub mod attacker;
pub mod defender;
pub mod judge;
pub mod synthesizer;

use anyhow::Result;

pub struct PipelineResult {
    pub claim: String,
    pub attack: String,
    pub defense: String,
    pub judgment: judge::Judgment,
    pub verdict: Verdict,
    pub reasoning: String,
}

impl PipelineResult {
    /// Format the full debate transcript and verdict for display.
    /// Shared by the CLI and the MCP tool response.
    pub fn format_output(&self) -> String {
        let survived_label = if self.judgment.survived {
            "survived"
        } else {
            "did not survive"
        };

        let verdict_line = match self.verdict {
            Verdict::Supported => "SUPPORTED — claim survived adversarial pressure",
            Verdict::Refuted   => "REFUTED — claim did not survive",
            Verdict::Paused    => "PAUSED — insufficient evidence to reach verdict",
        };

        let mut out = String::new();
        out.push_str("\n=== ArgumentClinic Analysis ===\n\n");
        out.push_str(&format!("CLAIM:\n{}\n\n", self.claim));
        out.push_str(&format!("ATTACK:\n{}\n\n", self.attack));
        out.push_str(&format!("DEFENSE:\n{}\n\n", self.defense));
        out.push_str(&format!(
            "JUDGMENT: {survived_label} (confidence: {:.2})\n{}\n\n",
            self.judgment.confidence,
            self.judgment.reason,
        ));
        out.push_str(&format!("VERDICT: {verdict_line}"));
        if !self.reasoning.is_empty() {
            out.push_str(&format!("\n{}", self.reasoning));
        }
        out
    }
}

pub enum Verdict {
    Supported,
    Refuted,
    /// Synthesizer paused — insufficient evidence to reach a verdict.
    Paused,
}

pub struct Pipeline {
    pub attacker: attacker::Attacker,
    pub defender: defender::Defender,
    pub judge: judge::Judge,
    pub synthesizer: synthesizer::Synthesizer,
}

impl Pipeline {
    pub fn new(
        attacker: attacker::Attacker,
        defender: defender::Defender,
        judge: judge::Judge,
        synthesizer: synthesizer::Synthesizer,
    ) -> Self {
        Self { attacker, defender, judge, synthesizer }
    }

    pub async fn run(&self, claim: &str) -> Result<PipelineResult> {
        let attack = self.attacker.attack(claim).await?;
        let defense = self.defender.defend(claim, &attack).await?;
        let judgment = self.judge.evaluate(claim, &attack, &defense).await?;
        let synthesis = self
            .synthesizer
            .synthesize(claim, &attack, &defense, &judgment)
            .await?;

        Ok(PipelineResult {
            claim: claim.to_string(),
            attack,
            defense,
            verdict: synthesis.verdict,
            reasoning: synthesis.reasoning,
            judgment,
        })
    }
}
