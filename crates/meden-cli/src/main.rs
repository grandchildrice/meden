//! Meden conversational coding agent.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use meden_runtime::{process::Control, workspace::Workspace};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{atomic::AtomicBool, Arc};
mod exports;
mod pages;
mod project;
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
    /// Fetch a fixed host-selected documentation batch to JSON on stdout.
    /// Run before confidential work; pass the saved file with --pages.
    Prefetch {
        /// Exact HTTPS request URL; repeat for up to eight pages.
        #[arg(long, required = true)]
        url: Vec<String>,
    },
    /// Check Meden release archives against exact-source GitHub CI evidence.
    VerifyRelease {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        directory: PathBuf,
    },
    /// Check a Rust ledger module against the fixed ledger-v1 contract locally.
    VerifyLedger {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    #[command(hide = true)]
    WorkerLedgerSyntax,
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
    /// Explicit HTTP connection ID, or "auto" to discover connections.
    #[arg(long)]
    provider: Option<String>,
    /// Model identifier for that connection.
    #[arg(long)]
    model: Option<String>,
    /// Authorize this exact inference URL to receive this session's context
    /// for up to eight hours. Required for run; optional in the TUI.
    #[arg(long, value_name = "URL")]
    approve_processor: Option<String>,
    /// Prepared llama-server executable. Defaults to MEDEN_LOCAL_ENGINE.
    #[arg(long, conflicts_with_all = ["provider", "model", "approve_processor"])]
    local_engine: Option<PathBuf>,
    /// Prepared GGUF weights. Defaults to MEDEN_LOCAL_MODEL.
    #[arg(long, conflicts_with_all = ["provider", "model", "approve_processor"])]
    local_model: Option<PathBuf>,
}

impl ModelArgs {
    fn local_config(&self) -> Result<Option<meden_llm::LocalConfig>> {
        if self.provider.is_some() {
            return Ok(None);
        }
        anyhow::ensure!(
            self.model.is_none() && self.approve_processor.is_none(),
            "HTTP model/approval requires an explicit --provider"
        );
        let engine = self
            .local_engine
            .clone()
            .or_else(|| std::env::var_os("MEDEN_LOCAL_ENGINE").map(PathBuf::from));
        let model = self
            .local_model
            .clone()
            .or_else(|| std::env::var_os("MEDEN_LOCAL_MODEL").map(PathBuf::from));
        Ok(Some(meden_llm::LocalConfig {
            engine: engine.context("configure --local-engine and --local-model (or MEDEN_LOCAL_ENGINE/MEDEN_LOCAL_MODEL); HTTP inference requires explicit --provider")?,
            model: model.context("local inference requires prepared GGUF weights via --local-model or MEDEN_LOCAL_MODEL")?,
        }))
    }
}
#[derive(Args)]
struct WorkspaceArgs {
    #[command(flatten)]
    model: ModelArgs,
    /// Workspace to read and edit.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    /// Host-supplied documentation JSON, loaded once before the session.
    #[arg(long, value_name = "FILE")]
    pages: Option<PathBuf>,
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
        Command::WorkerLedgerSyntax => {
            let source = read_ledger(io::stdin().lock())?;
            meden_runtime::verification::validate_ledger_source(&source)?;
        }
        Command::VerifyLedger {
            source,
            bundle,
            output,
        } => verify_ledger(&source, &bundle, &output)?,
        Command::VerifyRelease {
            repository,
            commit,
            directory,
        } => {
            verify_release(&repository, &commit, &directory)?;
        }
        Command::Prefetch { url } => {
            let pages = meden_runtime::pages::Pages::prefetch(&url)?;
            serde_json::to_writer(io::stdout().lock(), &pages)?;
            println!();
        }
        Command::Exec { workspace } => execute(&workspace)?,
        Command::WorkerFile => worker_file::run()?,
        Command::WorkerImport => meden_runtime::workspace::import_tree(
            std::path::Path::new("/source"),
            std::path::Path::new("/workspace"),
        )?,
        Command::Providers => print_providers(),
        Command::Tui(args) => {
            let pages = load_pages(args.pages.as_deref())?;
            let local = args.model.local_config()?;
            let providers = if local.is_some() {
                Vec::new()
            } else if args.model.provider.as_deref() == Some("auto") {
                meden_llm::detect()
            } else {
                vec![pick_provider(
                    args.model.provider.as_deref(),
                    args.model.model.as_deref(),
                )?]
            };
            tui::run(
                providers,
                &args.workspace,
                args.model.approve_processor.as_deref(),
                pages,
                local,
            )?;
        }
        Command::Run(args) => {
            let pages = load_pages(args.pages.as_deref())?;
            let local = args.model.local_config()?;
            let approved = if local.is_none() {
                Some(args.model.approve_processor.as_deref().context(
                "run requires --approve-processor with the exact /chat/completions URL; this permits sending conversation and tool results to that processor for this session (up to eight hours)",
            )?)
            } else {
                None
            };
            let mut input = io::BufReader::new(io::stdin());
            let first = protocol::read_command(&mut input)?.context("no command received")?;
            let workspace = Arc::new(Workspace::import(
                &args.workspace,
                &std::env::current_exe()?,
            )?);
            let processes = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
            let session = workspace.path().as_os_str().as_encoded_bytes();
            let gateway = Arc::new(if let Some(config) = local {
                meden_llm::Gateway::prepare_local(
                    session,
                    &config,
                    &std::env::current_exe()?,
                    Arc::clone(&processes),
                )?
            } else {
                let provider =
                    pick_provider(args.model.provider.as_deref(), args.model.model.as_deref())?;
                let gateway = meden_llm::Gateway::prepare(session, provider)?;
                approve_processor(&gateway, approved.context("missing processor approval")?)?;
                gateway
            });
            protocol::run(
                first,
                &mut input,
                &mut io::stdout().lock(),
                &workspace,
                gateway,
                &processes,
                &pages,
            )?;
        }
    }
    Ok(())
}

fn print_providers() {
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

fn verify_release(repository: &str, commit: &str, directory: &std::path::Path) -> Result<()> {
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .ok();
    let report = meden_runtime::release::verify(repository, commit, directory, token.as_deref())?;
    serde_json::to_writer(io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

fn read_ledger(mut input: impl io::Read) -> Result<String> {
    let mut bytes = Vec::new();
    input.by_ref().take(65_537).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 65_536, "ledger source exceeds 64 KiB");
    Ok(String::from_utf8(bytes)?)
}

fn verify_ledger(
    source: &std::path::Path,
    bundle: &std::path::Path,
    output: &std::path::Path,
) -> Result<()> {
    let source = read_ledger(std::fs::File::open(source)?)?;
    let bundle = meden_runtime::verification::Bundle::load(bundle)?;
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let checked = meden_runtime::verification::verify_ledger(
        &source,
        &bundle,
        &std::env::current_exe()?,
        &control,
    )?;
    let artifact = checked.persist(output)?;
    serde_json::to_writer(io::stdout().lock(), &artifact)?;
    println!();
    Ok(())
}

fn load_pages(path: Option<&std::path::Path>) -> Result<Arc<meden_runtime::pages::Pages>> {
    Ok(Arc::new(match path {
        Some(path) => meden_runtime::pages::Pages::load(std::fs::File::open(path)?)?,
        None => meden_runtime::pages::Pages::default(),
    }))
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
        Some(wanted) if wanted != "auto" => found
            .into_iter()
            .find(|p| p.id == wanted)
            .with_context(|| format!("no provider called {wanted:?}; try `meden providers`"))?,
        _ => found
            .into_iter()
            .next()
            .context("no model found; try `meden providers`")?,
    };
    if let Some(m) = model {
        m.clone_into(&mut provider.model);
    }
    Ok(provider)
}
