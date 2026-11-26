//! Host-only cold/refresh/resume cache diagnostic through the ordinary file worker.
use anyhow::{Context as _, Result};
use meden_app::{
    adapters::{navigation::Syntax, navigation_io, source},
    navigation::{cache, collection},
};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read as _,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct Reader<'a> {
    workspace: &'a Workspace,
    control: &'a Control,
    started: Instant,
}
impl cache::Reader for Reader<'_> {
    type Symbol = source::Symbol;
    type Stamp = serde_json::Value;
    fn stamps(&self, paths: &[String]) -> Result<Vec<Self::Stamp>> {
        navigation_io::Reader::new(self.workspace, self.control, self.started).stamps(paths)
    }
    fn sources(
        &self,
        paths: &[String],
        hints: &[&Self::Stamp],
    ) -> Result<Vec<collection::File<Self::Symbol>>> {
        collection::sources(
            &navigation_io::Reader::new(self.workspace, self.control, self.started)
                .with_hints(paths, hints)?,
            &Syntax,
            paths,
            || {},
        )
    }
    fn active(&self) -> Result<()> {
        Ok(self.control.with_active(|| Ok(()))?)
    }
}
#[derive(Deserialize, Serialize)]
struct Record {
    context: Context,
    parent: Option<u64>,
    name: String,
    start_us: u64,
    wall_us: u64,
    thread_cpu_us: Option<u64>,
}
#[derive(Deserialize, Serialize)]
struct Context {
    trace: u64,
    span: u64,
}
#[derive(Deserialize)]
struct Snapshot {
    evicted: u64,
    records: Vec<Record>,
}
fn observe() -> Result<Snapshot> {
    Ok(serde_json::from_value(serde_json::to_value(
        meden_runtime::metrics::snapshot(),
    )?)?)
}
struct Stop<'a>(&'a AtomicBool);
impl Drop for Stop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
fn collect(done: &AtomicBool) -> Result<Vec<Record>> {
    let mut records = BTreeMap::new();
    loop {
        let finished = done.load(Ordering::SeqCst);
        let snapshot = observe()?;
        let total = snapshot
            .evicted
            .checked_add(u64::try_from(snapshot.records.len())?)
            .context("Record count overflow")?;
        for record in snapshot.records {
            records.entry(record.context.span).or_insert(record);
        }
        anyhow::ensure!(
            records.len() <= 1_000_000,
            "Diagnostic record limit exceeded"
        );
        if finished {
            anyhow::ensure!(
                u64::try_from(records.len())? == total,
                "Incomplete metric capture; no timing report admitted"
            );
            return Ok(records.into_values().collect());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn paths(file: &Path) -> Result<Vec<String>> {
    const LIMIT: u64 = 67_108_864;
    let mut bytes = Vec::new();
    std::fs::File::open(file)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "Path list exceeds 64 MiB"
    );
    let paths: Vec<String> = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        !paths.is_empty()
            && paths.len() <= 250_000
            && paths.windows(2).all(|p| matches!(p,[a,b] if a < b)),
        "Expected complete ordered source paths"
    );
    anyhow::ensure!(
        paths.iter().all(|p| source::supports(Path::new(p))),
        "Expected Lean/Rust corpus; finite source conformance is separate"
    );
    Ok(paths)
}
#[derive(Deserialize)]
struct Expected {
    path: String,
    file: Option<collection::File<source::Symbol>>,
    error: Option<String>,
}
fn compare(
    store: &cache::Store<source::Symbol, serde_json::Value>,
    paths: &[String],
    expected: &Path,
) -> Result<String> {
    let mut digest = Sha256::new();
    for path in paths {
        let key = format!("{:x}", Sha256::digest(path.as_bytes()));
        let file = expected
            .join(key.get(..2).context("Invalid digest")?)
            .join(format!("{key}.json"));
        let original: Expected =
            meden_app::storage::read(&file)?.context("Original cache record missing")?;
        anyhow::ensure!(
            original.path == *path && original.error.is_none(),
            "Original record unavailable"
        );
        let original = original.file.context("Original file missing")?;
        let actual = store.load(path)?.context("Prepared cache record missing")?;
        anyhow::ensure!(
            actual == original,
            "Complete navigation fields changed for {path}"
        );
        let bytes = serde_json::to_vec(&actual)?;
        digest.update(u64::try_from(bytes.len())?.to_le_bytes());
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--help") {
        println!("Usage: profile-navigation-cache WORKSPACE WORKER PATHS_JSON_FILE EXPECTED_CACHE_DIRECTORY sampling|quiet\nUse a private complete source copy and fresh private XDG_DATA_HOME. Runs cold preparation, explicit unchanged refresh and sealed resume through the ordinary offline worker. Complete original File values must match after every phase. Sampling refuses incomplete captures; quiet mode calibrates observer overhead. No compiler, inference or evidence admission.");
        return Ok(());
    }
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .context("Expected private source workspace")?,
    )
    .canonicalize()?;
    let worker =
        PathBuf::from(arguments.next().context("Expected native worker")?).canonicalize()?;
    let list =
        PathBuf::from(arguments.next().context("Expected complete path list")?).canonicalize()?;
    let expected = PathBuf::from(
        arguments
            .next()
            .context("Expected original cache directory")?,
    )
    .canonicalize()?;
    let mode = arguments.next().context("Expected sampling or quiet")?;
    anyhow::ensure!(
        arguments.next().is_none() && matches!(mode.to_str(), Some("sampling" | "quiet")),
        "Use WORKSPACE WORKER PATHS_JSON_FILE EXPECTED_CACHE_DIRECTORY sampling|quiet"
    );
    let paths = paths(&list)?;
    let indexer = source::identity();
    let namespace = meden_runtime::workspace::state_directory(&root)?
        .join("map")
        .join(&indexer);
    anyhow::ensure!(
        !namespace.exists(),
        "Cold namespace already exists; use fresh private host storage"
    );
    let done = AtomicBool::new(false);
    let result = std::thread::scope(|threads| -> Result<_> {
        let observer = (mode == "sampling").then(|| threads.spawn(|| collect(&done)));
        let stop = Stop(&done);
        let measured = (|| -> Result<_> {
            let started = Instant::now();
            let workspace = Workspace::open(&root, &worker)?;
            let setup_ms = started.elapsed().as_secs_f64() * 1000.0;
            let control = Control::new(Arc::new(AtomicBool::new(false)));
            let reader = Reader {
                workspace: &workspace,
                control: &control,
                started: Instant::now(),
            };
            let store = cache::Store::host(&root, &indexer, |_| None)?;
            let mut samples = Vec::new();
            let mut original_digest = None;
            for (phase, fresh) in [("cold", false), ("refresh", true), ("resume", false)] {
                let started = Instant::now();
                store.prepare(&reader, &paths, fresh, cache::Scope::Inventory, |_| {})?;
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                let digest = compare(&store, &paths, &expected)?;
                if let Some(original) = &original_digest {
                    anyhow::ensure!(original == &digest, "Cache contents changed between phases");
                }
                original_digest = Some(digest.clone());
                samples.push(json!({"phase":phase,"elapsed_ms":elapsed_ms,"files":paths.len(),"complete_file_values_sha256":digest,"all_original_fields_equal":true,"generation":store.generation()?}));
                eprintln!("Completed {phase}");
            }
            Ok((setup_ms, samples))
        })();
        drop(stop);
        let records = match observer {
            Some(observer) => Some(
                observer
                    .join()
                    .map_err(|_| anyhow::anyhow!("Metric observer stopped"))??,
            ),
            None => None,
        };
        let (setup_ms, samples) = measured?;
        Ok(
            json!({"setup_ms":setup_ms,"samples":samples,"sampling":mode=="sampling","records":records}),
        )
    })?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"measurement":result,"worker_sha256":format!("{:x}",Sha256::digest(std::fs::read(worker)?)),"indexer_sha256":indexer,"paths_sha256":format!("{:x}",Sha256::digest(std::fs::read(list)?)),"scope":"Complete actual-source private corpus and original cached File values, actual confined workers and unchanged policy/bounds. Cold, unchanged explicit refresh and sealed resume; setup and complete output comparison excluded from intervals. Physical metadata stamps belong to the private copy and are not compared to original inode hints. No compiler, model, accepted evidence or cold-install claim. Sampling is diagnostic and must be calibrated against quiet runs; thread CPU excludes child processes and parallel/nested wall intervals overlap."})
        )?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missed_runtime_records_refuse_the_diagnostic_instead_of_reporting_zero() {
        for _ in 0..5000 {
            let _span = meden_runtime::metrics::Span::start("diagnostic.missed");
        }
        assert!(collect(&AtomicBool::new(true)).is_err());
    }
}
