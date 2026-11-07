//! `tagent attest lean` against this repository's own Lean project.
//!
//! Skipped when `lake` is not on PATH (the `rust` CI job); runs in the `lean`
//! CI job, where the project is already built.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command as Std;

use assert_cmd::Command;
use predicates::prelude::*;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lake_available() -> bool {
    Std::new("lake")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn lean_provider_yields_a_verified_attestation_the_gate_accepts() {
    if !lake_available() {
        eprintln!("attest: lake not on PATH; skipping");
        return;
    }
    let root = repo_root();
    let skill = root.join("examples/skills/csv-summary");
    let policy = root.join("examples/policies/read-only.json");
    let out = tempfile::tempdir().unwrap();
    let att = out.path().join("att.json");

    Command::cargo_bin("tagent")
        .unwrap()
        .args([
            "attest",
            "lean",
            "--skill",
            skill.to_str().unwrap(),
            "--project",
            root.join("lean").to_str().unwrap(),
            "--module",
            "TAgentSpec.Policy",
            "--theorem",
            "TAgentSpec.Policy.deny_wins",
            "--property",
            "inv.v0.deny-wins",
            "--statement",
            root.join("lean/TAgentSpec/Policy.lean").to_str().unwrap(),
            "--out",
            att.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("tier: verified"));

    let text = std::fs::read_to_string(&att).unwrap();
    assert!(text.contains("\"result\": \"proved\""));
    assert!(text.contains("\"backend\": \"lean4\""));

    Command::cargo_bin("tagent")
        .unwrap()
        .args([
            "gate",
            skill.to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
            "--attestation",
            att.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("admitted: true"));
}

#[test]
fn lean_provider_rejects_unknown_theorem() {
    if !lake_available() {
        return;
    }
    let root = repo_root();
    Command::cargo_bin("tagent")
        .unwrap()
        .args([
            "attest",
            "lean",
            "--skill",
            root.join("examples/skills/csv-summary").to_str().unwrap(),
            "--project",
            root.join("lean").to_str().unwrap(),
            "--module",
            "TAgentSpec.Policy",
            "--theorem",
            "TAgentSpec.Policy.no_such_theorem",
            "--property",
            "inv.v0.x",
            "--statement",
            root.join("lean/TAgentSpec/Policy.lean").to_str().unwrap(),
            "--no-build",
        ])
        .assert()
        .code(1);
}
