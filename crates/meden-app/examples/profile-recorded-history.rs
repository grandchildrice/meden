//! Host-only timing of complete portable records through the original file worker.
//! Output contains identities/counts/timings, never the document or credentials.
use anyhow::{Context as _, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--help") {
        println!("Usage: profile-recorded-history WORKSPACE WORKER SAMPLES\nWORKER is an installed Meden executable; SAMPLES is 1–16.\nReads the original complete .meden/workspace.json through the confined, policy-bound file worker. Does not edit files, execute Lean, approve inference or admit evidence. Output excludes record contents. Workspace/tool preparation and result hashing are outside the read interval; project/OS caches are not reset.");
        return Ok(());
    }
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(arguments.next().context("Expected workspace")?).canonicalize()?;
    let worker =
        PathBuf::from(arguments.next().context("Expected Meden worker")?).canonicalize()?;
    let samples: u8 = arguments
        .next()
        .context("Expected sample count")?
        .to_str()
        .context("Sample count must be UTF-8")?
        .parse()?;
    anyhow::ensure!(
        arguments.next().is_none() && (1..=16).contains(&samples),
        "Use WORKSPACE WORKER SAMPLES (1–16)"
    );
    anyhow::ensure!(root.is_dir(), "Workspace must be an existing directory");
    let started = Instant::now();
    let workspace = Workspace::open(&root, &worker)?;
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let mut observations = Vec::new();
    for sample in 0..samples {
        let started = Instant::now();
        let document = meden_app::records::store::load(&workspace, &control)?
            .context("Expected an original portable workspace document")?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let bytes = serde_json::to_vec(&document)?;
        observations.push(json!({"sample":sample,"elapsed_ms":elapsed_ms,
            "document_sha256":format!("sha256:{:x}",Sha256::digest(&bytes)),"canonical_bytes":bytes.len(),
            "tasks":document.tasks.len(),"events":document.events.len(),"conversations":document.conversations.len()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"workspace_preparation_ms":preparation_ms,
            "samples":observations,"metrics":meden_runtime::metrics::snapshot(),
            "scope":"Actual confined file worker and full portable record decoding/validation. No inference, Lean check, evidence admission, project edits, cold-install or renderer measurement. Workspace preparation and result hashing are outside the read interval; caches are not reset."}))?
    );
    Ok(())
}
