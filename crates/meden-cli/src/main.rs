//! Meden conversational coding agent.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
mod protocol;
mod shell;
mod tui;
mod workspace;

#[derive(Parser)]
#[command(name = "meden", version, about = "Conversational coding agent")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// List available HTTP model connections.
    Providers,
    /// Open a conversational workspace agent.
    Tui(TuiArgs),
    /// Run one conversation using a host's tools over JSON Lines.
    Run(ModelArgs),
}
#[derive(Args)]
struct ModelArgs {
    /// Connection ID from `meden providers`.
    #[arg(long)]
    provider: Option<String>,
    /// Model identifier for that connection.
    #[arg(long)]
    model: Option<String>,
}
#[derive(Args)]
struct TuiArgs {
    #[command(flatten)]
    model: ModelArgs,
    /// Workspace to read and edit.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    /// Enable unrestricted development shell execution (not sandboxed).
    #[arg(long)]
    allow_shell: bool,
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Providers => {
            for p in meden_llm::detect() {
                println!("{:<22} {} · {} · {}", p.id, p.label, p.model, p.note);
            }
        }
        Command::Tui(args) => {
            let providers = if args.model.provider.is_some() || args.model.model.is_some() {
                vec![pick_provider(
                    args.model.provider.as_deref(),
                    args.model.model.as_deref(),
                )?]
            } else {
                meden_llm::detect()
            };
            tui::run(providers, &args.workspace, args.allow_shell)?;
        }
        Command::Run(args) => {
            let mut input = io::BufReader::new(io::stdin());
            let start = protocol::read_start(&mut input)?;
            let provider = pick_provider(args.provider.as_deref(), args.model.as_deref())?;
            protocol::run(&start, &provider, Box::new(input), Box::new(io::stdout()))?;
        }
    }
    Ok(())
}
/// Select a model connection: the one asked for, or the first one detected.
fn pick_provider(id: Option<&str>, model: Option<&str>) -> Result<meden_llm::Provider> {
    let found = meden_llm::detect();
    let mut provider = match id {
        Some(wanted) => found
            .into_iter()
            .find(|p| p.id == wanted)
            .with_context(|| format!("no provider called {wanted:?}; try `meden providers`"))?,
        None => found
            .into_iter()
            .next()
            .context("no model found; try `meden providers`")?,
    };
    if let Some(m) = model {
        m.clone_into(&mut provider.model);
    }
    Ok(provider)
}
