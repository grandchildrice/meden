//! Discover installed host resources without invoking Elan or workspace executables.
use anyhow::{Context, Result};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub(super) fn discover(workspace: &Path) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("MEDEN_ATLAS_LEAN") {
        return distribution(Path::new(&path))
            .context("The host's MEDEN_ATLAS_LEAN does not select an installed Lean distribution");
    }
    let home = std::env::var_os("ELAN_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".elan")));
    discover_in(workspace, home.as_deref(), std::env::var_os("PATH"))
}

fn discover_in(workspace: &Path, elan: Option<&Path>, path: Option<OsString>) -> Result<PathBuf> {
    let workspace = workspace.canonicalize()?;
    let pin = workspace.join("lean-toolchain");
    match fs::symlink_metadata(&pin) {
        Ok(_) => {
            anyhow::ensure!(
                pin.canonicalize()?.starts_with(&workspace),
                "lean-toolchain must remain inside the workspace"
            );
            let selected = bounded_text(&pin)?;
            return installed(&selected, elan);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if let Some(elan) = elan {
        let settings = elan.join("settings.toml");
        if settings.exists() {
            for line in bounded_text(&settings)?
                .lines()
                .take_while(|line| !line.trim().starts_with('['))
            {
                if let Some((key, value)) = line.split_once('=') {
                    if key.trim() == "default_toolchain" {
                        // Elan writes this field as a quoted string. Never execute its value.
                        let selected: String = serde_json::from_str(value.trim())
                            .context("Cannot read Elan's default_toolchain setting")?;
                        return installed(&selected, Some(elan));
                    }
                }
            }
        }
    }
    if let Some(path) = path {
        for entry in std::env::split_paths(&path) {
            // Ignore relative/repository paths and the Elan proxy. No PATH executable runs here.
            if !entry.is_absolute() {
                continue;
            }
            let Ok(bin) = entry.canonicalize() else {
                continue;
            };
            if bin.starts_with(&workspace) {
                continue;
            }
            let Ok(executable) = bin.join("lean").canonicalize() else {
                continue;
            };
            if executable.starts_with(&workspace) {
                continue;
            }
            if let Some(root) = executable.parent().and_then(Path::parent) {
                if let Ok(root) = distribution(root) {
                    return Ok(root);
                }
            }
        }
    }
    anyhow::bail!("Installed Lean was not found. Install Lean with Elan, then choose 再検出. No environment variable is required.")
}

fn installed(selected: &str, elan: Option<&Path>) -> Result<PathBuf> {
    let selected = selected.trim();
    let version = selected
        .strip_prefix("leanprover/lean4:")
        .context("lean-toolchain must name a leanprover/lean4 version")?;
    anyhow::ensure!(
        !version.is_empty()
            && version.len() <= 100
            && version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b)),
        "Invalid Lean toolchain version"
    );
    let missing = || {
        format!("Project Lean {selected} is not installed. Run `elan toolchain install {selected}`, then choose 再検出; no restart or environment variable is required.")
    };
    let elan = elan.with_context(missing)?;
    anyhow::ensure!(elan.is_absolute(), "Elan's host directory must be absolute");
    distribution(
        &elan
            .join("toolchains")
            .join(format!("leanprover--lean4---{version}")),
    )
    .with_context(missing)
}

fn bounded_text(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 65_536,
        "Lean selection must be a bounded regular file"
    );
    fs::read_to_string(path).context("Cannot read Lean selection")
}

fn distribution(path: &Path) -> Result<PathBuf> {
    let root = path.canonicalize()?;
    anyhow::ensure!(
        root.join("bin/lean").is_file()
            && root.join("bin/lake").is_file()
            && root.join("lib/lean/Init.olean").is_file()
            && root.join("lib/lean/libLake_shared.so").is_file(),
        "Incomplete Lean distribution: {}",
        root.display()
    );
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path, version: &str) -> Result<PathBuf> {
        let root = root
            .join("toolchains")
            .join(format!("leanprover--lean4---{version}"));
        fs::create_dir_all(root.join("bin"))?;
        fs::create_dir_all(root.join("lib/lean"))?;
        for name in [
            "bin/lean",
            "bin/lake",
            "lib/lean/Init.olean",
            "lib/lean/libLake_shared.so",
        ] {
            fs::write(root.join(name), "fixture, never executed")?;
        }
        Ok(root)
    }
    #[test]
    fn project_pin_wins_and_missing_version_never_uses_another() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let elan = temp.path().join("elan");
        fs::create_dir(&workspace)?;
        let wanted = fixture(&elan, "v4.36.0")?;
        fixture(&elan, "v4.35.0")?;
        fs::write(
            elan.join("settings.toml"),
            "default_toolchain = \"leanprover/lean4:v4.35.0\"\n",
        )?;
        fs::write(
            workspace.join("lean-toolchain"),
            "leanprover/lean4:v4.36.0\n",
        )?;
        assert_eq!(discover_in(&workspace, Some(&elan), None)?, wanted);
        fs::remove_dir_all(wanted)?;
        let error = discover_in(&workspace, Some(&elan), None)
            .err()
            .context("Missing project toolchain was accepted")?;
        assert!(error
            .to_string()
            .contains("elan toolchain install leanprover/lean4:v4.36.0"));
        fs::write(
            workspace.join("lean-toolchain"),
            "leanprover/lean4:../../bad\n",
        )?;
        assert!(discover_in(&workspace, Some(&elan), None).is_err());
        Ok(())
    }
    #[test]
    fn no_pin_uses_elan_default_or_public_path_without_running_proxies() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let elan = temp.path().join("elan");
        fs::create_dir(&workspace)?;
        let wanted = fixture(&elan, "v4.36.0")?;
        fs::write(
            elan.join("settings.toml"),
            "default_toolchain = \"leanprover/lean4:v4.36.0\"\n[overrides]\n",
        )?;
        assert_eq!(discover_in(&workspace, Some(&elan), None)?, wanted);
        let shadow = fixture(&workspace, "v4.35.0")?;
        let path = std::env::join_paths([shadow.join("bin"), wanted.join("bin")])?;
        assert_eq!(discover_in(&workspace, None, Some(path))?, wanted);
        Ok(())
    }
}
