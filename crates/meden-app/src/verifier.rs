//! Host-only first-launch preparation. This runs before importing a workspace,
//! never as a model tool or from candidate configuration.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use meden_runtime::verification::Bundle;
use serde_json::json;

const SETUP: &str = include_str!("../../../tools/setup-verifier.sh");
const CACHE_VERSION: &str = "lean-4.35.0-aeneas-c5eb4a2-rust-2026-09-17";

pub fn manifest() -> Option<PathBuf> {
    std::env::var_os("MEDEN_VERIFIER_BUNDLE")
        .map(PathBuf::from)
        .or_else(|| storage().map(|root| root.join("bundle.json")))
}

fn storage() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .filter(|path| path.is_absolute())
        .map(|path| path.join("meden/verifier").join(CACHE_VERSION))
}

/// Prepare or reuse the verifier before any confidential project is imported.
/// An explicit host override is authoritative and never silently replaced.
pub fn prepare() -> Result<PathBuf> {
    let path = manifest().context("No local storage is available for verification tools")?;
    if std::env::var_os("MEDEN_VERIFIER_BUNDLE").is_some() {
        Bundle::load(&path)
            .context("The explicitly selected verifier installation could not be loaded")?;
        return Ok(path);
    }
    if Bundle::load(&path).is_ok() {
        return Ok(path);
    }
    if path.exists() {
        eprintln!("Repairing the saved local verification installation…");
    }
    let root = path.parent().context("missing verification storage")?;
    std::fs::create_dir_all(root)?;
    let _lock = setup_lock(root)?;
    if Bundle::load(&path).is_ok() {
        return Ok(path);
    }
    // Only these embedded public package definitions enter the preparer. The
    // user's working directory and its Cargo/Lake settings are never read.
    seed_libraries(root)?;
    eprintln!(
        "Preparing local verification tools for the first launch. This may take several minutes."
    );
    eprintln!("Only public tool downloads run now; your project has not been opened.");
    let home = std::env::var_os("HOME").context("No home directory is available")?;
    let setup_home = tempfile::Builder::new()
        .prefix("setup-home.")
        .tempdir_in(root)?;
    let bash = [
        "/run/current-system/sw/bin/bash",
        "/usr/bin/bash",
        "/bin/bash",
    ]
    .into_iter()
    .map(Path::new)
    .find(|path| path.is_file())
    .context("Automatic verification setup needs Bash installed on the host")?;
    let mut command = Command::new(bash);
    command
        .args(["--noprofile", "--norc", "-s", "--"])
        .arg(root)
        .arg(&home)
        // A source build may reuse its already built, pinned public libraries.
        // This is the binary's build location, never the current project.
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lean"))
        .current_dir(root)
        .env_clear()
        .env("PATH", "/run/current-system/sw/bin:/usr/bin:/bin")
        .env("HOME", setup_home.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("CI", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    // Explicit host toolchain locations remain trusted inputs. No credentials,
    // shell startup files or model/provider configuration are inherited.
    for name in ["RUSTUP_HOME", "CARGO_HOME", "ELAN_HOME"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .context("missing setup input")?
        .write_all(SETUP.as_bytes())?;
    let result = child.wait_with_output()?;
    anyhow::ensure!(result.status.success(), "Could not prepare verification tools. Check the setup message above and restart Meden; completed downloads are reused.");
    let output = String::from_utf8(result.stdout)?;
    let mut lines = output.lines();
    let rust = lines.next().context("setup did not return Rust tools")?;
    let lean = lines.next().context("setup did not return Lean tools")?;
    let extraction = lines
        .next()
        .context("setup did not return extraction tools")?;
    let libraries: Vec<_> = lines.filter(|line| !line.is_empty()).collect();
    let manifest = json!({
        "version": 1, "rust": rust, "lean": lean,
        "charon": Path::new(extraction).join("charon"),
        "driver": Path::new(extraction).join("charon-driver"),
        "aeneas": Path::new(extraction).join("aeneas"),
        "libraries": libraries,
    });
    let mut staged = tempfile::NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(&mut staged, &manifest)?;
    staged.flush()?;
    let bundle =
        Bundle::load(staged.path()).context("The prepared verification tools are incomplete")?;
    self_check(&bundle)?;
    staged
        .persist(&path)
        .map_err(|error| io::Error::other(error.error))?;
    eprintln!("Local verification tools are ready. Future launches reuse them.");
    Ok(path)
}

fn seed_libraries(root: &Path) -> Result<()> {
    let project = root.join("libraries");
    std::fs::create_dir_all(&project)?;
    for (name, contents) in [
        ("lakefile.toml", include_str!("../../../lean/lakefile.toml")),
        (
            "lake-manifest.json",
            include_str!("../../../lean/lake-manifest.json"),
        ),
        (
            "lean-toolchain",
            include_str!("../../../lean/lean-toolchain"),
        ),
    ] {
        std::fs::write(project.join(name), contents)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn setup_lock(root: &Path) -> Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("setup.lock"))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)?;
    Ok(file)
}

#[cfg(not(target_os = "linux"))]
fn setup_lock(_root: &Path) -> Result<std::fs::File> {
    anyhow::bail!("Automatic verifier preparation requires Linux")
}

fn self_check(bundle: &Bundle) -> Result<()> {
    eprintln!("Checking the local verifier on a public example…");
    let control = meden_runtime::process::Control::new(std::sync::Arc::new(
        std::sync::atomic::AtomicBool::new(false),
    ));
    let contract = meden_runtime::verification::Contract::parse(include_str!(
        "../../../verification/examples/counter.contract.json"
    ))?;
    meden_runtime::verification::kernel::prepare_kernel(
        include_str!("../../../verification/examples/counter.rs"),
        &contract,
        bundle,
        &std::env::current_exe()?,
        &control,
    )?
    .check(&control)
    .context("The public verifier self-check failed; no project has been opened")?;
    Ok(())
}
