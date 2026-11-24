//! Host-only diagnostic of the actual confined Lake preparation/fingerprint.
//! No graph, model call, accepted contract or evidence admission is performed.
use anyhow::{Context as _, Result};
use meden_runtime::process::Control;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
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
        println!("Usage: profile-prepared-inputs WORKSPACE BINDINGS_JSON SAMPLES\nBINDINGS_JSON is an array of original Lake root name/file/root objects; SAMPLES is 1–16.\nRuns actual offline preparation and complete compiled-input validation. It does not reset project/OS caches, extract a graph, invoke a model or admit evidence.");
        return Ok(());
    }
    let mut arguments = std::env::args().skip(1);
    let root = PathBuf::from(arguments.next().context("Expected workspace")?).canonicalize()?;
    let bindings = arguments
        .next()
        .context("Expected selected root bindings JSON")?;
    let samples: u8 = arguments.next().context("Expected sample count")?.parse()?;
    anyhow::ensure!(
        arguments.next().is_none() && (1..=16).contains(&samples),
        "Use WORKSPACE BINDINGS_JSON SAMPLES (1–16)"
    );
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let diagnostic = Arc::new(Mutex::new(None::<Value>));
    let captured = Arc::clone(&diagnostic);
    let configuration = meden_app::atlas::Configuration::host(&root).with_observer(Some(Arc::new(
        move |phase, _, output| {
            if let Some(output) = output {
                if let Ok(mut last) = captured.lock() {
                    *last = Some(json!({"stdout":output.stdout,"stderr":output.stderr,"truncated":output.truncated}));
                }
            } else {
                eprintln!("{phase}");
            }
        },
    )));
    let started = Instant::now();
    configuration.ensure_prepared(&control)?;
    let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let resources = configuration.current_resources()?;
    let python = meden_runtime::sandbox::Sandbox::public_tool("python3");
    let mut observations = Vec::new();
    for sample in 0..samples {
        if let Ok(mut last) = diagnostic.lock() {
            *last = None;
        }
        let started = Instant::now();
        let result = configuration.project(
            &root,
            "project-atlas-prepare.sh",
            &["prepare".into(), bindings.clone()],
            &control,
        )?;
        let observation = json!({
            "sample":sample,"elapsed_ms":started.elapsed().as_secs_f64()*1000.0,
            "result":serde_json::from_str::<Value>(&result)?,
            "diagnostic":diagnostic.lock().map_err(|_| anyhow::anyhow!("Diagnostic unavailable"))?.as_ref()
        });
        eprintln!("Completed sample {sample}");
        observations.push(observation);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"resources_revision":resources,"public_python":python,"resource_preparation_ms":preparation_ms,"samples":observations,"metrics":meden_runtime::metrics::snapshot(),"scope":"Actual installed Lake, original confined offline preparation and complete compiled-input bytes. Project/OS caches are not reset; resource preparation is measured separately. No inference, graph extraction, current proof, cold installation or physical presentation measurement."})
        )?
    );
    Ok(())
}
