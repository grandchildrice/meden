//! Time the actual offline source/layout selection without semantic preparation.
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
    if std::env::args_os()
        .nth(1)
        .is_some_and(|argument| argument == "worker-launch")
    {
        anyhow::ensure!(
            std::env::args_os()
                .nth(2)
                .is_some_and(|argument| argument == "--"),
            "Expected launcher separator"
        );
        meden_runtime::process::exec_clean(&std::env::args_os().skip(3).collect::<Vec<_>>())?;
        anyhow::bail!("Launcher unexpectedly returned");
    }
    if std::env::args().nth(1).as_deref() == Some("--help") {
        println!("Usage: profile-source-selection WORKSPACE WORKER PATHS_JSON SAMPLES\nWORKER is an installed Meden executable; PATHS_JSON is the original selected file array; SAMPLES is 1–16.\nRuns original policy-bound source reads and frozen offline Lake configuration evaluation. Does not build project modules, extract graphs, run search/inference or admit evidence. Resource preparation and complete result hashing are outside the selection interval. Project/OS caches are not reset; output excludes source/configuration contents.");
        return Ok(());
    }
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(arguments.next().context("Expected workspace")?).canonicalize()?;
    let worker =
        PathBuf::from(arguments.next().context("Expected Meden worker")?).canonicalize()?;
    let paths = arguments.next().context("Expected selected paths JSON")?;
    let paths: Vec<String> = serde_json::from_str(paths.to_str().context("Paths must be UTF-8")?)?;
    let samples: u8 = arguments
        .next()
        .context("Expected sample count")?
        .to_str()
        .context("Sample count must be UTF-8")?
        .parse()?;
    anyhow::ensure!(
        arguments.next().is_none() && (1..=16).contains(&samples),
        "Use WORKSPACE WORKER PATHS_JSON SAMPLES (1–16)"
    );
    let started = Instant::now();
    let workspace = Workspace::open(&root, &worker)?;
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let atlas = meden_app::atlas::Configuration::host(&root);
    atlas.ensure_prepared(&control)?;
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let resources = atlas.current_resources()?;
    let mut observations = Vec::new();
    for sample in 0..samples {
        let started = Instant::now();
        let selected =
            meden_app::analysis::collect::roots(&workspace, &control, &paths, atlas.clone())?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let bytes = serde_json::to_vec(&selected)?;
        let collection = selected
            .collection
            .as_ref()
            .context("Missing original collection")?;
        observations.push(json!({"sample":sample,"elapsed_ms":elapsed_ms,
            "selection_sha256":format!("sha256:{:x}",Sha256::digest(&bytes)),
            "canonical_bytes":bytes.len(),"source_files":selected.source.files.len(),
            "packages":collection.packages.len(),"configuration_observations":collection.metadata.files.len(),
            "configuration_bytes":collection.metadata.files.values().flatten().map(String::len).sum::<usize>()}));
        eprintln!("Completed source-selection sample {sample}");
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"resources_revision":resources,
        "workspace_resource_preparation_ms":preparation_ms,"samples":observations,
        "metrics":meden_runtime::metrics::snapshot(),
        "scope":"Actual confined source reads and complete frozen Lake layout selection. No project-module build, graph, search, inference, evidence admission or cold installation. Complete selection identities exclude no original source/configuration fields. Preparation and result hashing are outside selection intervals; caches are not reset."}))?
    );
    Ok(())
}
