//! Meden conversational coding agent.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use meden_runtime::{process::Control, workspace::Workspace};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{atomic::AtomicBool, Arc};
mod protocol;
mod session;
mod shell;
mod text;
mod tui;
mod worker_file;
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
    Tui(WorkspaceArgs),
    /// Run instructions from JSON Lines with private offline workspace tools.
    Run(WorkspaceArgs),
    /// Run Bash from stdin in a new private offline workspace.
    Exec {
        /// Project to import; the original is not modified.
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    #[command(hide = true)]
    WorkerFile,
    #[command(hide = true)]
    WorkerImport,
    #[command(hide = true)]
    WorkerLaunch {
        #[arg(last = true, required = true)]
        arguments: Vec<std::ffi::OsString>,
    },
}
#[derive(Args)]
struct ModelArgs {
    /// Connection ID from `meden providers`.
    #[arg(long)]
    provider: Option<String>,
    /// Model identifier for that connection.
    #[arg(long)]
    model: Option<String>,
    /// Authorize this exact inference URL to receive this session's context
    /// for up to eight hours. Required for run; optional in the TUI.
    #[arg(long, value_name = "URL")]
    approve_processor: Option<String>,
}
#[derive(Args)]
struct WorkspaceArgs {
    #[command(flatten)]
    model: ModelArgs,
    /// Workspace to read and edit.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(
                io::stderr(),
                "error: {}",
                text::visible(&format!("{error:#}"))
            );
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Command::WorkerLaunch { arguments } => meden_runtime::process::exec_clean(&arguments)?,
        Command::Exec { workspace } => execute(&workspace)?,
        Command::WorkerFile => worker_file::run()?,
        Command::WorkerImport => meden_runtime::workspace::import_tree(
            std::path::Path::new("/source"),
            std::path::Path::new("/workspace"),
        )?,
        Command::Providers => {
            for p in meden_llm::detect() {
                println!(
                    "{}",
                    text::visible(&format!(
                        "{:<22} {} · {} · {}",
                        p.id, p.label, p.model, p.note
                    ))
                );
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
            tui::run(
                providers,
                &args.workspace,
                args.model.approve_processor.as_deref(),
            )?;
        }
        Command::Run(args) => {
            let approved = args.model.approve_processor.as_deref().context(
                "run requires --approve-processor with the exact /chat/completions URL; this permits sending conversation and tool results to that processor for this session (up to eight hours)",
            )?;
            let mut input = io::BufReader::new(io::stdin());
            let first =
                protocol::read_instruction(&mut input)?.context("no instruction received")?;
            let provider =
                pick_provider(args.model.provider.as_deref(), args.model.model.as_deref())?;
            let workspace = Arc::new(Workspace::import(
                &args.workspace,
                &std::env::current_exe()?,
            )?);
            let gateway = Arc::new(meden_llm::Gateway::prepare(
                workspace.path().as_os_str().as_encoded_bytes(),
                provider,
            )?);
            approve_processor(&gateway, approved)?;
            let processes = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
            protocol::run(
                first,
                &mut input,
                &mut io::stdout().lock(),
                workspace,
                gateway,
                &processes,
            )?;
        }
    }
    Ok(())
}

fn approve_processor(gateway: &meden_llm::Gateway, approved: &str) -> Result<()> {
    anyhow::ensure!(
        approved == gateway.endpoint(),
        "processor approval does not match the prepared endpoint {}",
        gateway.endpoint()
    );
    gateway.approve_from_host()?;
    Ok(())
}

fn execute(source: &std::path::Path) -> Result<()> {
    use meden_agent::tool::Args;

    let mut command = String::new();
    io::stdin().take(16_777_217).read_to_string(&mut command)?;
    anyhow::ensure!(command.len() <= 16_777_216, "command exceeds input budget");
    let workspace = Arc::new(Workspace::import(source, &std::env::current_exe()?)?);
    let control = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
    let (registry, _policy) = workspace::environment(&workspace, &control)?;
    let tool = registry.get("bash").context("missing Bash tool")?;
    let args: Args = [("command".to_owned(), serde_json::json!(command))]
        .into_iter()
        .collect();
    let result = tool.call(&args);
    let record = match &result {
        Ok(output) => serde_json::json!({"workspace":workspace.path(), "result":output}),
        Err(error) => serde_json::json!({"workspace":workspace.path(), "error":error.to_string()}),
    };
    println!("{}", serde_json::to_string(&record)?);
    result?;
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
