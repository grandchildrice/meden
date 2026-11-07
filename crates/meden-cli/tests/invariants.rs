//! The shipped catalogue must describe the Lean sources as they are.
//!
//! A property id in an attestation is only worth something if it refers to a
//! fixed statement. This is what keeps it fixed: weaken a theorem and the
//! entry a human approved no longer matches, here, in the same pull request.

// Tests fail by panicking; that is their job.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("the crate is two levels below the repository root")
}

#[test]
fn every_catalogued_invariant_still_says_what_it_said() {
    let root = repo_root();
    let out = Command::new(env!("CARGO_BIN_EXE_meden"))
        .args(["invariants", "--explain"])
        .current_dir(&root)
        .output()
        .expect("running meden invariants");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "a catalogued statement no longer matches its Lean source.\n\
         Read the new statement. If it still says what the entry claims, \
         update the hash in the same commit; if it does not, the entry was \
         approved for something else.\n\n{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The two the whole design rests on must be in there.
    for id in [
        "inv.v0.untrusted-data-never-steers",
        "inv.v0.no-disclosure-to-a-non-reader",
    ] {
        assert!(text.contains(id), "{id} is not catalogued:\n{text}");
    }
}

#[test]
fn a_property_the_catalogue_does_not_know_is_refused() {
    let root = repo_root();
    let out = Command::new(env!("CARGO_BIN_EXE_meden"))
        .args([
            "attest",
            "lean",
            "--artifact-sha256",
            &"0".repeat(64),
            "--project",
            "lean",
            "--module",
            "MedenSpec.Flow",
            "--theorem",
            "untrusted_control_refused",
            "--property",
            "inv.v0.invented-on-the-spot",
            "--catalogue",
            "invariants/catalogue.json",
            "--no-build",
        ])
        .current_dir(&root)
        .output()
        .expect("running meden attest lean");
    assert!(
        !out.status.success(),
        "an uncatalogued property id was attested without complaint"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("not in"), "the reason was not given: {err}");
}

#[test]
fn a_property_pointing_at_the_wrong_theorem_is_refused() {
    let root = repo_root();
    let out = Command::new(env!("CARGO_BIN_EXE_meden"))
        .args([
            "attest",
            "lean",
            "--artifact-sha256",
            &"0".repeat(64),
            "--project",
            "lean",
            "--module",
            "MedenSpec.Policy",
            "--theorem",
            "deny_wins",
            // This id is catalogued, but against a different theorem.
            "--property",
            "inv.v0.untrusted-data-never-steers",
            "--catalogue",
            "invariants/catalogue.json",
            "--no-build",
        ])
        .current_dir(&root)
        .output()
        .expect("running meden attest lean");
    assert!(
        !out.status.success(),
        "a theorem was attested under another property's id"
    );
}
