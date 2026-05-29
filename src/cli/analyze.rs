use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::{
    config,
    pipeline::{
        Pipeline,
        attacker::Attacker,
        defender::Defender,
        judge::Judge,
        synthesizer::Synthesizer,
    },
    providers::build_provider,
};

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// Read the claim from a file
    #[arg(long, conflicts_with = "text")]
    pub file: Option<PathBuf>,

    /// Use this string directly as the claim to analyze
    #[arg(long, conflicts_with = "file")]
    pub text: Option<String>,
}

pub async fn run(args: AnalyzeArgs) -> Result<()> {
    // ── config ────────────────────────────────────────────────────────────────
    let cfg = config::load().context("failed to load config")?;

    // ── role assignments ──────────────────────────────────────────────────────
    let roles = &cfg.roles;

    macro_rules! require_role {
        ($field:ident, $name:literal) => {
            roles.$field.as_ref().ok_or_else(|| {
                anyhow!(
                    "no {name} role configured\n\n\
                     Run: argumentclinic config set-role {name} <provider> <model>",
                    name = $name
                )
            })?
        };
    }

    let attacker_a = require_role!(attacker, "attacker");
    let defender_a = require_role!(defender, "defender");
    let judge_a = require_role!(judge, "judge");
    let synthesizer_a = require_role!(synthesizer, "synthesizer");

    // ── provider construction ─────────────────────────────────────────────────
    let attacker = Attacker::new(
        build_provider(attacker_a, &cfg.providers)
            .context("failed to build attacker provider")?,
    );
    let defender = Defender::new(
        build_provider(defender_a, &cfg.providers)
            .context("failed to build defender provider")?,
    );
    let judge = Judge::new(
        build_provider(judge_a, &cfg.providers)
            .context("failed to build judge provider")?,
    );
    let synthesizer = Synthesizer::new(
        build_provider(synthesizer_a, &cfg.providers)
            .context("failed to build synthesizer provider")?,
    );

    // ── input ─────────────────────────────────────────────────────────────────
    let input = if let Some(path) = args.file {
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
    } else if let Some(text) = args.text {
        text
    } else {
        eprintln!("Reading from stdin (Ctrl+D to finish)…");
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        buf
    };

    let input = input.trim().to_string();
    if input.is_empty() {
        return Err(anyhow!("input is empty — nothing to analyze"));
    }

    // ── run ───────────────────────────────────────────────────────────────────
    eprintln!("Running adversarial pipeline…");
    let pipeline = Pipeline::new(attacker, defender, judge, synthesizer);
    let r = pipeline.run(&input).await?;

    // ── output ────────────────────────────────────────────────────────────────
    println!("{}", r.format_output());
    Ok(())
}
