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
mod catalogue;
mod demo;
mod protocol;
mod tui;

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
    /// List the models this machine could plan with.
    Providers,
    /// Open the interface: pick a model, give it an instruction, watch the
    /// gate decide.
    Tui,
    /// Run one session with the tools owned by another process.
    ///
    /// Reads a JSON opening message on standard input and then speaks JSON
    /// lines both ways. This is how a benchmark harness drives the runtime.
    Run(RunArgs),
    /// Ask a model for a plan and print exactly what it said.
    ///
    /// For working out why a model's plans are rejected.
    Raw(DemoArgs),
    /// Check the invariant catalogue against the Lean sources.
    ///
    /// An attestation names a property by id. This says what each id means
    /// and whether the theorem it points at still states it.
    Invariants(InvariantArgs),
}

#[derive(Args)]
struct InvariantArgs {
    /// The catalogue file.
    #[arg(long, default_value = "invariants/catalogue.json")]
    catalogue: PathBuf,
    /// The directory the catalogue's paths are relative to.
    #[arg(long, default_value = "lean")]
    lean: PathBuf,
    /// Print what each invariant means, not only whether it matches.
    #[arg(long)]
    explain: bool,
}

#[derive(Args)]
struct DemoArgs {
    /// Ask a real model for the plans instead of using scripted ones.
    ///
    /// Without --provider, the first model `tagent providers` lists is used.
    #[arg(long)]
    live: bool,
    /// Which model to plan with, by the id `tagent providers` shows.
    #[arg(long)]
    provider: Option<String>,
    /// The model name, when the provider needs one choosing.
    #[arg(long)]
    model: Option<String>,
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

#[derive(Args)]
struct RunArgs {
    /// Print how the declared tools read back, then stop.
    ///
    /// For checking a host's declarations without running anything.
    #[arg(long)]
    describe: bool,
    /// Which model to plan with, by the id `tagent providers` shows.
    #[arg(long)]
    provider: Option<String>,
    /// The model name, when the provider needs one choosing.
    #[arg(long)]
    model: Option<String>,
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
        Command::Raw(a) => run_raw(&a, &mut out),
        Command::Invariants(a) => check_invariants(&a, &mut out),
        Command::Providers => list_providers(&mut out),
        Command::Run(a) => run_protocol(&a, &mut out),
        Command::Tui => {
            let providers = tagent_llm::detect();
            if std::env::var(tui::ENV_SELFTEST).is_ok() {
                writeln!(out, "{}", tui::selftest(providers)?)?;
                return Ok(ExitCode::SUCCESS);
            }
            tui::run(providers)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Demo(a) => {
            if a.live {
                let provider = pick_provider(a.provider.as_deref(), a.model.as_deref())?;
                writeln!(
                    io::stderr(),
                    "planning with {} ({})",
                    provider.label,
                    provider.model
                )?;
                demo::run_with(&provider, Some(&provider), &mut out)?;
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

/// Run one session with the tools owned by another process.
fn run_protocol(a: &RunArgs, out: &mut impl Write) -> Result<ExitCode> {
    let start = {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        protocol::read_start(&mut input)?
    };
    if a.describe {
        for (tool, notes) in protocol::describe(&start) {
            writeln!(out, "{tool}: {}", notes.join(", "))?;
        }
        return Ok(ExitCode::SUCCESS);
    }
    let provider = pick_provider(a.provider.as_deref(), a.model.as_deref())?;
    protocol::run(
        &start,
        &provider,
        Some(&provider),
        Box::new(io::BufReader::new(io::stdin())),
        Box::new(io::stdout()),
    )?;
    Ok(ExitCode::SUCCESS)
}

/// Ask a model for a plan and print exactly what it said.
fn run_raw(a: &DemoArgs, out: &mut impl Write) -> Result<ExitCode> {
    let provider = pick_provider(a.provider.as_deref(), a.model.as_deref())?;
    let registry = demo::registry();
    let tools = registry
        .names()
        .filter_map(|n| registry.get(n).map(|t| t.signature().clone()))
        .collect();
    let request = tagent_agent::PlanRequest::new(demo::INSTRUCTION, tools);
    let reply = provider
        .chat(
            &tagent_agent::system_prompt(&request),
            &tagent_agent::user_prompt(&request),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    writeln!(out, "--- {} characters ---", reply.chars().count())?;
    writeln!(out, "{reply}")?;
    Ok(ExitCode::SUCCESS)
}

/// Print what this machine could plan with.
fn list_providers(out: &mut impl Write) -> Result<ExitCode> {
    let found = tagent_llm::detect();
    if found.is_empty() {
        writeln!(out, "nothing found.")?;
        writeln!(
            out,
            "install one of claude, codex, hermes or ollama, or set {}.",
            tagent_llm::ENV_BASE_URL
        )?;
        return Ok(exit_for(false));
    }
    for p in &found {
        writeln!(out, "{:<22} {}", p.id, p.label)?;
        writeln!(out, "{:<22} model {}  ({})", "", p.model, p.note)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The model to plan with: the one asked for, or the first one detected.
fn pick_provider(id: Option<&str>, model: Option<&str>) -> Result<tagent_llm::Provider> {
    let found = tagent_llm::detect();
    let mut provider = match id {
        Some(wanted) => found
            .into_iter()
            .find(|p| p.id == wanted)
            .with_context(|| format!("no provider called {wanted:?}; try `tagent providers`"))?,
        None => found
            .into_iter()
            .next()
            .context("no model found; try `tagent providers`")?,
    };
    if let Some(m) = model {
        m.clone_into(&mut provider.model);
    }
    Ok(provider)
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

/// Report whether each catalogued invariant still says what it said.
///
/// A property id in an attestation is only worth something if it refers to a
/// fixed statement. This is the check that keeps it fixed: it re-reads the
/// Lean source and compares the hash, so weakening a theorem breaks the
/// entry that a human once approved instead of quietly inheriting its
/// approval.
fn check_invariants(a: &InvariantArgs, out: &mut impl Write) -> Result<ExitCode> {
    let catalogue = catalogue::Catalogue::load(&a.catalogue)?;
    let results = catalogue.verify(&a.lean);
    let mut bad = 0usize;
    for (entry, status) in &results {
        let note = match status {
            catalogue::Status::Matches => "ok".to_owned(),
            catalogue::Status::Changed { found } => {
                bad = bad.saturating_add(1);
                format!(
                    "CHANGED: the source now hashes to {}",
                    &found[..16.min(found.len())]
                )
            }
            catalogue::Status::Missing => {
                bad = bad.saturating_add(1);
                format!(
                    "MISSING: no theorem {} in {}",
                    entry.lean.theorem, entry.lean.file
                )
            }
            catalogue::Status::Unreadable(e) => {
                bad = bad.saturating_add(1);
                format!("UNREADABLE: {e}")
            }
        };
        writeln!(out, "{:<48} {note}", entry.id)?;
        if a.explain {
            writeln!(out, "    {}", entry.means)?;
            writeln!(out, "    {} :: {}", entry.lean.file, entry.lean.theorem)?;
        }
    }
    writeln!(
        out,
        "\n{} invariants, {} in order",
        results.len(),
        results.len().saturating_sub(bad)
    )?;
    if bad > 0 {
        writeln!(
            out,
            "A catalogued statement changed. Read the new one, and if it still \n\
             says what the entry claims, update the hash in the same commit."
        )?;
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}
