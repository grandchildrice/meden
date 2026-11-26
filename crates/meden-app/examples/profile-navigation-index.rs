//! Host-only lexical-index comparison over a frozen complete-text corpus.
//! Uses the original offline file policy; no compilation, inference or evidence admission.
use anyhow::{Context as _, Result};
use meden_app::adapters::source::{self, Symbol};
use meden_app::{
    adapters::{navigation::Syntax, navigation_io::Reader},
    navigation::collection::{self, Indexed, Indexer},
};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    hint::black_box,
    io::Read as _,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

#[allow(missing_docs)]
#[path = "../src/adapters/source/oracle.rs"]
mod original;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    files: Vec<Input>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: String,
    lean: bool,
    text: String,
}

// Read only the public diagnostic projection; runtime records stay opaque.
#[derive(Deserialize)]
struct ObservedMetrics {
    records: Vec<ObservedSpan>,
    evicted: u64,
}
#[derive(Deserialize)]
struct ObservedSpan {
    context: ObservedContext,
    name: String,
    thread_cpu_us: Option<u64>,
}
#[derive(Deserialize)]
struct ObservedContext {
    span: u64,
}
fn observations() -> Result<ObservedMetrics> {
    Ok(serde_json::from_value(serde_json::to_value(
        meden_runtime::metrics::snapshot(),
    )?)?)
}

fn reference(input: &Input) -> (usize, usize, Vec<String>, Vec<Symbol>) {
    let (count, theorems, imports) = original::headers(&input.text, input.lean);
    (
        count,
        theorems,
        imports,
        original::symbols(&input.text, input.lean),
    )
}
fn combined(input: &Input) -> (usize, usize, Vec<String>, Vec<Symbol>) {
    let value = source::index(&input.text, input.lean);
    (
        value.declarations,
        value.theorems,
        value.imports,
        value.symbols,
    )
}
fn run(corpus: &Corpus, combined_index: bool) -> f64 {
    let start = Instant::now();
    for input in &corpus.files {
        black_box(if combined_index {
            combined(input)
        } else {
            reference(input)
        });
    }
    start.elapsed().as_secs_f64() * 1000.0
}
struct Reference;
impl Indexer for Reference {
    type Symbol = Symbol;
    fn supports(&self, path: &str) -> bool {
        source::supports(Path::new(path))
    }
    fn index(&self, path: &str, text: &str) -> Result<Indexed<Symbol>> {
        let lean = Path::new(path).extension().is_some_and(|e| e == "lean");
        let (declarations, theorems, imports) = original::headers(text, lean);
        Ok(Indexed {
            declarations,
            theorems,
            imports,
            symbols: original::symbols(text, lean),
        })
    }
}
fn confined(
    corpus: &Corpus,
    workspace: &Workspace,
    control: &Control,
    combined_index: bool,
) -> Result<serde_json::Value> {
    let before = observations()?;
    let spans = before
        .records
        .iter()
        .map(|r| r.context.span)
        .collect::<std::collections::BTreeSet<_>>();
    let paths = corpus
        .files
        .iter()
        .map(|f| f.path.clone())
        .collect::<Vec<_>>();
    let started = Instant::now();
    let reader = Reader::new(workspace, control, started);
    let files = if combined_index {
        collection::sources(&reader, &Syntax, &paths, || {})?
    } else {
        collection::sources(&reader, &Reference, &paths, || {})?
    };
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    anyhow::ensure!(files.len() == corpus.files.len(), "Incomplete native index");
    for (file, input) in files.iter().zip(&corpus.files) {
        let (count, theorems, imports, symbols) = reference(input);
        anyhow::ensure!(
            file.indexed
                && file.path == input.path
                && file.bytes == input.text.len()
                && file.sha256 == format!("{:x}", Sha256::digest(input.text.as_bytes()))
                && file.declarations == count
                && file.theorems == theorems
                && file.imports == imports
                && file.symbols == symbols,
            "Native worker observation changed for {}",
            input.path
        );
    }
    let after = observations()?;
    let indices = after
        .records
        .iter()
        .filter(|r| r.name == "navigation.index" && !spans.contains(&r.context.span))
        .collect::<Vec<_>>();
    anyhow::ensure!(
        indices.len() == files.len(),
        "Native index spans incomplete"
    );
    let cpu = indices.iter().try_fold(0u64, |sum, r| {
        sum.checked_add(r.thread_cpu_us.context("Index CPU unavailable")?)
            .context("Index CPU overflow")
    })?;
    Ok(
        json!({"combined":combined_index,"native_collection_ms":elapsed_ms,"index_thread_cpu_us":cpu,"index_spans":indices.len(),"complete_outputs_equal":true,"ring_evicted":after.evicted.saturating_sub(before.evicted)}),
    )
}
fn main() -> Result<()> {
    const LIMIT: u64 = 512 * 1024 * 1024;
    let mut args = std::env::args().skip(1);
    if std::env::args().nth(1).as_deref() == Some("--help") {
        println!("Usage: profile-navigation-index CORPUS_JSON MEDEN_WORKER\nCompares original/combined lexical indices in memory, then through the ordinary offline file worker in a fresh private copy. All source bytes and output fields must match. No compilation, inference or evidence admission.");
        return Ok(());
    }
    let path = args
        .next()
        .context("Use profile-navigation-index CORPUS_JSON MEDEN_WORKER")?;
    let worker = std::fs::canonicalize(args.next().context("Expected native Meden worker")?)?;
    anyhow::ensure!(
        args.next().is_none(),
        "Use a frozen complete-text corpus and fixed worker"
    );
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "Corpus exceeds 512 MiB"
    );
    let corpus: Corpus = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        !corpus.files.is_empty() && corpus.files.len() <= 250_000,
        "Invalid corpus count"
    );
    let mut names = std::collections::BTreeSet::new();
    let copy = tempfile::tempdir()?;
    for input in &corpus.files {
        anyhow::ensure!(
            !input.path.is_empty() && names.insert(&input.path),
            "Duplicate/empty corpus path"
        );
        anyhow::ensure!(
            reference(input) == combined(input),
            "Lexical observations changed for {}",
            input.path
        );
        let relative = meden_app::workspace::observation::relative("", &input.path)?;
        anyhow::ensure!(
            relative == input.path
                && input.lean
                    == Path::new(&input.path)
                        .extension()
                        .is_some_and(|e| e == "lean"),
            "Invalid corpus path/mode"
        );
        let target = copy.path().join(relative);
        std::fs::create_dir_all(target.parent().context("Missing parent")?)?;
        std::fs::write(target, input.text.as_bytes())?;
    }
    // All output comparison, input I/O and both warmups stay outside timings.
    run(&corpus, false);
    run(&corpus, true);
    let samples = (0..12).map(|order| {
        let combined_index = matches!(order % 4, 1 | 2);
        json!({"order":order,"combined":combined_index,"index_and_drop_ms":run(&corpus, combined_index)})
    }).collect::<Vec<_>>();
    let setup = Instant::now();
    let workspace = Workspace::open(copy.path(), &worker)?;
    let native_setup_ms = setup.elapsed().as_secs_f64() * 1000.0;
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    confined(&corpus, &workspace, &control, false)?;
    confined(&corpus, &workspace, &control, true)?;
    let native_samples = [false, true, true, false]
        .into_iter()
        .map(|combined_index| confined(&corpus, &workspace, &control, combined_index))
        .collect::<Result<Vec<_>>>()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "corpus_sha256":format!("{:x}",Sha256::digest(&bytes)),
            "files":corpus.files.len(),"source_bytes":corpus.files.iter().map(|f|f.text.len()).sum::<usize>(),
            "complete_outputs_equal":true,"samples":samples,"native_samples":native_samples,"native_setup_ms":native_setup_ms,
            "worker_sha256":format!("{:x}",Sha256::digest(std::fs::read(worker)?)),
            "scope":"Frozen UTF-8 corpus, exact counts/imports/ordered symbol fields against the independent original two-pass implementation. Standalone index plus drop excludes input I/O, equivalence and warmups. Native samples use the same actual confined file worker, four-reader complete source collection and unchanged policy/limits on a private complete-text copy; they include I/O and indexing, but exclude setup/output comparison and warmups. OS/tool caches remain warm. Neither measures complete GUI navigation, compilation, proof, cold installation or live inference."
        }))?
    );
    Ok(())
}
