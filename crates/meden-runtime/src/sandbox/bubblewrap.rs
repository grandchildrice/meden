//! Locate an installed launcher without changing PATH or installing software.
//! Only fixed system locations and their public Nix software closures are
//! additional discovery sources; repository configuration is never consulted.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use super::{find_public, host_command, public_path, store_root};
use crate::process::{self, Control, Limits, Termination};

const BIN_DIRS: &[&str] = &[
    "/usr/bin",
    "/bin",
    "/usr/local/bin",
    "/run/current-system/sw/bin",
    "/nix/var/nix/profiles/default/bin",
];
const NIX_PROFILES: &[&str] = &["/run/current-system/sw", "/nix/var/nix/profiles/default"];

pub(super) fn discover() -> io::Result<PathBuf> {
    // An explicit host override remains authoritative, including when invalid.
    // Never silently replace it with a different launcher after failure.
    if let Some(path) = std::env::var_os("MEDEN_BWRAP") {
        return PathBuf::from(path).canonicalize();
    }
    if let Some(path) = find_public("bwrap").or_else(|| fixed_program("bwrap")) {
        return Ok(path);
    }
    if let Some(path) = nix_bubblewrap()? {
        return Ok(path);
    }
    Err(io::Error::other(
        "bubblewrap is required for isolated execution: install Bubblewrap with your system package manager, or set MEDEN_BWRAP to a trusted executable; installed public system/Nix locations are detected automatically",
    ))
}

fn public_file(path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    (path.is_file() && public_path(&path)).then_some(path)
}

fn fixed_program(name: &str) -> Option<PathBuf> {
    BIN_DIRS
        .iter()
        .find_map(|directory| public_file(&Path::new(directory).join(name)))
}

fn nix_bubblewrap() -> io::Result<Option<PathBuf>> {
    let Some(nix_store) = find_public("nix-store").or_else(|| fixed_program("nix-store")) else {
        return Ok(None);
    };
    for profile in NIX_PROFILES {
        let Ok(root) = Path::new(profile).canonicalize() else {
            continue;
        };
        if store_root(&root) != Some(root.as_path()) || !root.is_dir() {
            continue;
        }
        let mut command = host_command(&nix_store, "nix-store")?;
        command.args(["--query", "--requisites"]).arg(root);
        // This is a local metadata query, without realise/build/substitution.
        // Even a failed or oversized query cannot choose an unchecked path.
        let output = process::run(
            &mut command,
            &Limits {
                timeout: Duration::from_secs(10),
                capture_bytes: 4_194_304,
                output_bytes: 8_388_608,
            },
            &Control::new(Arc::new(AtomicBool::new(false))),
        )?;
        if !matches!(output.termination, Termination::Exited(status) if status.success())
            || output.truncated
        {
            return Err(io::Error::other(
                "cannot inspect installed system software for Bubblewrap",
            ));
        }
        let closure = parse_closure(&output.stdout)?;
        for root in &closure {
            if let Some(candidate) = public_file(&root.join("bin/bwrap")) {
                if store_root(&candidate).is_some_and(|root| closure.contains(root)) {
                    return Ok(Some(candidate));
                }
            }
        }
    }
    Ok(None)
}

fn parse_closure(text: &str) -> io::Result<BTreeSet<PathBuf>> {
    text.lines()
        .map(|line| {
            let path = PathBuf::from(line);
            if path.parent() != Some(Path::new("/nix/store")) || path.file_name().is_none() {
                return Err(io::Error::other("unexpected system software closure path"));
            }
            Ok(path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn malformed_or_non_store_metadata_cannot_choose_a_launcher() {
        for text in [
            "/tmp/repository\n",
            "/nix/store/pkg/bin\n",
            "/nix/store/..\n",
            "relative\n",
        ] {
            assert!(parse_closure(text).is_err(), "{text}");
        }
        let paths =
            parse_closure("/nix/store/a-bubblewrap\n/nix/store/b-system-path\n").expect("closure");
        assert_eq!(paths.len(), 2);
    }
}
