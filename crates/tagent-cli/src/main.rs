//! `tagent`: the command-line face of the gate.
//!
//! Exit codes: `0` allowed / verified, `2` denied / not verified, `1` error.

#![forbid(unsafe_code)]
#![allow(clippy::exit)]

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

mod attest;
mod demo;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tagent_core::{check, Attestation, Decision, Policy, Requirement, Tier};
use tagent_skill::{extract_requirements, load_skill};

const EXIT_DENIED: u8 = 2;

#[derive(Parser)]
#[command(
    name = "tagent",
    version,
    about = "Capability-typed gate for LLM agent skills and tool calls"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read a skill package and list the capabilities it needs.
    Extract(ExtractArgs),
    /// Check a skill package against a policy.
    Check(CheckArgs),
    /// Decide a requirement set against a policy (JSON on stdin).
    Decide,
    /// Classify an attestation file.
    Tier(TierArgs),
    /// Admission: policy check plus attestation tier for a skill package.
    Gate(GateArgs),
    /// Print the content digest of a skill package.
    Digest(ExtractArgs),
    /// Produce an attestation with a verifier plugin.
    #[command(subcommand)]
    Attest(AttestCommand),
    /// Run the prompt-injection scenario end to end.
    Demo(DemoArgs),
}

#[derive(Args)]
struct DemoArgs {
    /// Ask a real model for the plans instead of using scripted ones.
    ///
    /// Reads TAGENT_LLM_BASE_URL and TAGENT_LLM_MODEL, and optionally
    /// TAGENT_LLM_API_KEY. Any OpenAI-compatible server will do: vLLM,
    /// Ollama or llama.cpp.
    #[arg(long)]
    live: bool,
}

#[derive(Subcommand)]
enum AttestCommand {
    /// Attest a theorem in a Lean 4 (Lake) project.
    Lean(attest::LeanArgs),
}

#[derive(Args)]
struct ExtractArgs {
    /// Skill directory (contains SKILL.md).
    dir: PathBuf,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct CheckArgs {
    /// Skill directory (contains SKILL.md).
    dir: PathBuf,
    /// Policy file (JSON).
    #[arg(long)]
    policy: PathBuf,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct TierArgs {
    /// Attestation file (JSON).
    file: PathBuf,
    /// Verifier backend whose property mapping a human has approved. Repeatable.
    #[arg(long = "approve")]
    approved: Vec<String>,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct GateArgs {
    /// Skill directory (contains SKILL.md).
    dir: PathBuf,
    /// Policy file (JSON).
    #[arg(long)]
    policy: PathBuf,
    /// Attestation file (JSON). Repeatable.
    #[arg(long = "attestation")]
    attestations: Vec<PathBuf>,
    /// Verifier backend whose property mapping a human has approved. Repeatable.
    #[arg(long = "approve")]
    approved: Vec<String>,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

/// Input of `tagent decide`.
#[derive(Deserialize)]
struct DecideInput {
    requirements: Vec<Requirement>,
    policy: Policy,
}

/// Output of `tagent gate`.
#[derive(Serialize)]
struct GateReport {
    name: String,
    digest: String,
    requirements: Vec<Requirement>,
    policy: Decision,
    /// Best tier among attestations whose artifact hash matches the digest.
    evidence: Tier,
    /// Attestations that did not match the digest, with the reason.
    ignored: Vec<String>,
    /// `policy.allowed && evidence.is_verified()`.
    admitted: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(io::stderr(), "error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let mut out = io::stdout().lock();
    match cli.command {
        Command::Extract(a) => {
            let pkg = load_skill(&a.dir)?;
            let reqs = extract_requirements(&pkg);
            if a.json {
                writeln!(out, "{}", serde_json::to_string_pretty(&reqs)?)?;
            } else {
                writeln!(out, "{}: {} requirement(s)", pkg.name, reqs.len())?;
                for r in &reqs {
                    writeln!(out, "  {r}  <- {}", r.evidence)?;
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Digest(a) => {
            let pkg = load_skill(&a.dir)?;
            writeln!(out, "{}", pkg.digest())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check(a) => {
            let pkg = load_skill(&a.dir)?;
            let policy = read_policy(&a.policy)?;
            let decision = check(&extract_requirements(&pkg), &policy);
            print_decision(&mut out, &decision, a.json)?;
            Ok(exit_for(decision.allowed))
        }
        Command::Decide => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .context("reading stdin")?;
            let DecideInput {
                requirements,
                policy,
            } = serde_json::from_str(&input).context("parsing decide input")?;
            let decision = check(&requirements, &policy);
            writeln!(out, "{}", serde_json::to_string(&decision)?)?;
            Ok(exit_for(decision.allowed))
        }
        Command::Tier(a) => {
            let att = read_attestation(&a.file)?;
            let tier = att.tier(&a.approved);
            if a.json {
                writeln!(out, "{}", serde_json::to_string(&tier)?)?;
            } else {
                writeln!(out, "{}", tier_name(tier))?;
            }
            Ok(exit_for(tier.is_verified()))
        }
        Command::Gate(a) => run_gate(&a, &mut out),
        Command::Demo(a) => {
            if a.live {
                let client =
                    tagent_llm::OpenAiCompatible::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;
                writeln!(io::stderr(), "planning with {}", client.base_url())?;
                demo::run(&client, &mut out)?;
            } else {
                demo::run(&demo::scripted(), &mut out)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Attest(AttestCommand::Lean(a)) => {
            let (att, tier) = attest::run(&a)?;
            let json = serde_json::to_string_pretty(&att)?;
            match &a.out {
                Some(path) => fs::write(path, format!("{json}\n"))
                    .with_context(|| format!("writing {}", path.display()))?,
                None => writeln!(out, "{json}")?,
            }
            writeln!(io::stderr(), "tier: {}", tier_name(tier))?;
            Ok(exit_for(tier.is_verified()))
        }
    }
}

fn run_gate(a: &GateArgs, out: &mut impl Write) -> Result<ExitCode> {
    let pkg = load_skill(&a.dir)?;
    let policy = read_policy(&a.policy)?;
    let digest = pkg.digest();
    let requirements = extract_requirements(&pkg);
    let decision = check(&requirements, &policy);
    let mut evidence = Tier::NotVerified;
    let mut ignored = Vec::new();
    for path in &a.attestations {
        let att = read_attestation(path)?;
        if att.artifact.sha256 != digest {
            ignored.push(format!(
                "{}: artifact hash does not match package digest",
                path.display()
            ));
            continue;
        }
        let t = att.tier(&a.approved);
        evidence = best_tier(evidence, t);
    }
    let report = GateReport {
        name: pkg.name.clone(),
        digest,
        requirements,
        admitted: decision.allowed && evidence.is_verified(),
        policy: decision,
        evidence,
        ignored,
    };
    if a.json {
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        writeln!(out, "{} ({})", report.name, report.digest)?;
        writeln!(
            out,
            "policy:   {}",
            if report.policy.allowed {
                "allowed"
            } else {
                "denied"
            }
        )?;
        for r in &report.policy.reasons {
            writeln!(out, "  - {r}")?;
        }
        writeln!(out, "evidence: {}", tier_name(report.evidence))?;
        for i in &report.ignored {
            writeln!(out, "  - ignored {i}")?;
        }
        writeln!(out, "admitted: {}", report.admitted)?;
    }
    Ok(exit_for(report.admitted))
}

fn read_policy(path: &PathBuf) -> Result<Policy> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading policy {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing policy {}", path.display()))
}

fn read_attestation(path: &PathBuf) -> Result<Attestation> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading attestation {}", path.display()))?;
    Attestation::from_json(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

fn print_decision(out: &mut impl Write, d: &Decision, json: bool) -> Result<()> {
    if json {
        writeln!(out, "{}", serde_json::to_string(d)?)?;
    } else {
        writeln!(out, "{}", if d.allowed { "allowed" } else { "denied" })?;
        for r in &d.reasons {
            writeln!(out, "  - {r}")?;
        }
    }
    Ok(())
}

const fn tier_name(t: Tier) -> &'static str {
    match t {
        Tier::Verified => "verified",
        Tier::VerifiedWithTcb => "verified-with-tcb",
        Tier::NotVerified => "not-verified",
    }
}

const fn best_tier(a: Tier, b: Tier) -> Tier {
    match (a, b) {
        (Tier::Verified, _) | (_, Tier::Verified) => Tier::Verified,
        (Tier::VerifiedWithTcb, _) | (_, Tier::VerifiedWithTcb) => Tier::VerifiedWithTcb,
        _ => Tier::NotVerified,
    }
}

fn exit_for(ok: bool) -> ExitCode {
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_DENIED)
    }
}
