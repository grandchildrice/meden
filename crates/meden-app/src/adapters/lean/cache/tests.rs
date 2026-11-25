use super::*;
use crate::explanation::cache::artifact::{CounterexampleSearch, MeaningRendering};
use proptest::prelude::*;
use serde_json::json;
use std::fmt::Write as _;

fn observed() -> Result<Snapshot> {
    let files = BTreeMap::from([(
        "Spec.lean".to_string(),
        "theorem claim : False := by sorry\n".to_string(),
    )]);
    Ok(serde_json::from_value(json!({
        "revision":format!("sha256:{}", hash(&files)?), "files":files,
        "collection":{"roots":["Spec.lean"],"external_modules":[],"modules":[],"packages":[],"metadata":{"files":{},"directories":{}}},
        "atlas":{"revision":format!("sha256:{}", "1".repeat(64)),"upstream":"fixture","resources_sha256":"fixture",
            "graph":{"metadata":{"projectName":"cache fixture","leanVersion":"fixture"},"nodes":[{
                "id":"claim","shortName":"claim","kind":"theorem","filePath":"Spec.lean","lineStart":1,"lineEnd":1,
                "hasSorry":true,"statement":"False","value":null,"axioms":[],"standardAxioms":[],"externalDependencies":[],
                "semantic":{"version":1,"origin":"elaborated","sentence":"structural sentence","renderer":"Structural English",
                    "refutation":"original refutation","intermediate":"original formal expression",
                    "checker":{"engine":"fixture","status":"counterexample","candidates":["original witness"],"seed":42}}
            }],"edges":[],"externalReferences":[]}}
    }))?)
}
fn semantic(snapshot: &mut Snapshot) -> Result<&mut crate::explanation::Description> {
    snapshot
        .atlas
        .as_mut()
        .and_then(|g| g.graph.nodes.first_mut())
        .and_then(|n| n.semantic.as_mut())
        .context("semantic")
}
fn searched_id(observation: &Observation) -> Result<Id<CounterexampleSearch>> {
    match &observation.identity {
        Identity::Searched { search, .. } => Ok(search.clone()),
        Identity::Compilation(_) => anyhow::bail!("Expected search result"),
    }
}
fn revision(snapshot: &Snapshot) -> Result<&GraphRevision> {
    Ok(&snapshot.atlas.as_ref().context("graph")?.revision)
}

#[test]
fn renderer_change_reuses_raw_observations_and_binds_a_distinct_complete_view() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let original = observed()?;
    let id = store
        .observe(original.clone(), &definition()?)?
        .compilation()
        .clone();
    let mut first = original.clone();
    semantic(&mut first)?.sentence = "first renderer sentence".into();
    semantic(&mut first)?.renderer = Some("first renderer".into());
    store.render(
        store.observe(original.clone(), &definition()?)?,
        &renderer("renderer-one")?,
        &mut first,
    )?;
    let hit = store
        .rendered(&id, &original, &renderer("renderer-one")?)?
        .context("render cache miss")?;
    assert_eq!(serde_json::to_value(&first)?, serde_json::to_value(hit)?);
    assert_ne!(revision(&first)?, revision(&original)?);

    assert!(store
        .rendered(&id, &original, &renderer("renderer-two")?)?
        .is_none());
    let retained = store.observe(original.clone(), &definition()?)?;
    let mut next = retained.snapshot().clone();
    assert_eq!(
        serde_json::to_value(&next)?,
        serde_json::to_value(&original)?
    );
    // A failed or unavailable new renderer starts from the original structural
    // sentence, never the previous renderer's English attributed to a new one.
    semantic(&mut next)?.renderer = Some("Structural English (new renderer unavailable)".into());
    semantic(&mut next)?.rendering_error = Some("new renderer failed".into());
    store.render(retained, &renderer("renderer-two")?, &mut next)?;
    assert_ne!(revision(&next)?, revision(&first)?);
    assert_eq!(semantic(&mut next)?.sentence, "structural sentence");
    assert_eq!(
        semantic(&mut next)?.checker,
        semantic(&mut original.clone())?.checker
    );
    assert_eq!(
        semantic(&mut next)?.intermediate,
        semantic(&mut original.clone())?.intermediate
    );
    let retained: Artifact = store.read(&id)?;
    assert_eq!(
        serde_json::to_value(retained.snapshot)?,
        serde_json::to_value(original)?
    );
    Ok(())
}

#[test]
fn rendering_cannot_rewrite_check_results_sources_refutations_or_dependencies() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let original = observed()?;
    for (path, value) in [
        (
            "/atlas/graph/nodes/0/semantic/checker/status",
            json!("not_found"),
        ),
        (
            "/atlas/graph/nodes/0/semantic/checker/candidates",
            json!([]),
        ),
        (
            "/atlas/graph/nodes/0/semantic/refutation",
            json!("changed refutation"),
        ),
        (
            "/atlas/graph/nodes/0/semantic/intermediate",
            json!("changed expression"),
        ),
        ("/atlas/graph/nodes/0/hasSorry", json!(false)),
        ("/files/Spec.lean", json!("changed source")),
        ("/collection/roots", json!([])),
    ] {
        let mut value_snapshot = serde_json::to_value(&original)?;
        *value_snapshot.pointer_mut(path).context("mutation path")? = value;
        let mut changed: Snapshot = serde_json::from_value(value_snapshot)?;
        assert!(
            store
                .render(
                    store.observe(original.clone(), &definition()?)?,
                    &renderer("renderer")?,
                    &mut changed
                )
                .is_err(),
            "{path}"
        );
    }
    assert!(store.index::<Rendering>()?.is_empty());
    Ok(())
}

#[test]
fn partial_duplicate_foreign_or_unbounded_readings_are_never_published() -> Result<()> {
    let original = observed()?;
    let reading = || Reading {
        node: "claim".into(),
        sentence: "reading".into(),
        renderer: None,
        error: None,
    };
    for readings in [
        vec![],
        vec![reading(), reading()],
        vec![Reading {
            node: "foreign".into(),
            ..reading()
        }],
        vec![Reading {
            sentence: "x".repeat(65_537),
            ..reading()
        }],
    ] {
        let rendering = Rendering {
            observation: Id::of(&Artifact {
                version: 2,
                compiler: json!({}),
                snapshot: original.clone(),
            })?,
            renderer: serde_json::to_value(renderer("renderer")?)?,
            readings,
        };
        assert!(rendering.project(original.clone()).is_err());
    }
    assert!(serde_json::from_value::<Reading>(json!({"node":"claim","sentence":"reading","renderer":null,"error":null,"checker":{"status":"not_found"}})).is_err());
    Ok(())
}

#[test]
fn interrupted_rendering_keeps_reusable_raw_data_but_no_complete_view() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let original = observed()?;
    let store = Store(directory.path().into());
    let id = store
        .observe(original.clone(), &definition()?)?
        .compilation()
        .clone();
    // Restart with only the successfully persisted compiler/search artifact.
    let restarted = Store(directory.path().into());
    let artifact: Artifact = restarted.read(&id)?;
    assert!(restarted
        .rendered(&id, &artifact.snapshot, &renderer("selected-renderer")?)?
        .is_none());
    Ok(())
}

#[test]
fn corrupted_rendering_is_refused_without_losing_the_original_observation() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let mut rendered = observed()?;
    let original = store.observe(rendered.clone(), &definition()?)?;
    let id = original.compilation().clone();
    store.render(original, &renderer("renderer")?, &mut rendered)?;
    let rendering_id = store
        .index::<Rendering>()?
        .into_values()
        .next()
        .context("rendering id")?;
    let path = store
        .0
        .join("renderings")
        .join(format!("{rendering_id}.json"));
    let mut json: serde_json::Value = crate::storage::read(&path)?.context("rendering")?;
    *json
        .pointer_mut("/readings/0/sentence")
        .context("sentence")? = json!("modified content");
    crate::storage::write(&path, &json)?;
    assert!(store
        .rendered(&id, &observed()?, &renderer("renderer")?)
        .is_err());
    let original: Artifact = store.read(&id)?;
    original.check()?;
    Ok(())
}

#[test]
fn stale_observation_identity_cannot_select_another_rendering() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let mut first = observed()?;
    let original = store.observe(first.clone(), &definition()?)?;
    let id = original.compilation().clone();
    store.render(original, &renderer("renderer")?, &mut first)?;
    let first_rendering = store
        .index::<Rendering>()?
        .into_values()
        .next()
        .context("rendering id")?;
    let mut other = observed()?;
    other.source.files.insert(
        "Spec.lean".into(),
        "-- changed\ntheorem claim : False := by sorry\n".into(),
    );
    other.source.revision = SourceRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(
        &other.source.files,
    )?));
    let other_id = store
        .observe(other.clone(), &definition()?)?
        .compilation()
        .clone();
    assert_ne!(id, other_id);
    assert!(store
        .rendered(&other_id, &other, &renderer("renderer")?)?
        .is_none());
    let mut index = store.index::<Rendering>()?;
    index.insert(
        hash(&(&other_id, renderer("renderer")?.identity()))?,
        first_rendering,
    );
    crate::storage::write(&store.0.join("renderings-index.json"), &index)?;
    assert!(store
        .rendered(&other_id, &other, &renderer("renderer")?)
        .is_err());
    assert_eq!(
        scope(&["B.lean".into(), "A.lean".into()], false, &definition()?)?,
        scope(&["A.lean".into(), "B.lean".into()], false, &definition()?)?
    );
    assert_ne!(
        scope(&["A.lean".into()], false, &definition()?)?,
        scope(&["A.lean".into(), "B.lean".into()], false, &definition()?)?
    );
    assert_ne!(
        scope(&["A.lean".into()], false, &definition()?)?,
        scope(&["A.lean".into()], true, &definition()?)?
    );
    Ok(())
}

fn definition() -> Result<meden_runtime::plugin::Descriptor> {
    Ok(meden_runtime::plugin::Descriptor::new(
        meden_runtime::plugin::Definition {
            id: "fixture.compiler",
            role: meden_runtime::plugin::Role::Compiler,
            version: "1",
            formats: vec!["fixture".into()],
            implementation: b"original compiler",
            configuration: json!({"seed":42}),
            input: crate::compiler::input_schema()?,
            output: crate::compiler::output_schema::<crate::adapters::lean::Snapshot>()?,
        },
    )?)
}

fn renderer(id: &str) -> Result<meden_runtime::plugin::Descriptor> {
    Ok(meden_runtime::plugin::Descriptor::new(
        meden_runtime::plugin::Definition {
            id,
            role: meden_runtime::plugin::Role::MeaningRenderer,
            version: "1",
            formats: vec!["fixture".into()],
            implementation: b"original renderer",
            configuration: json!({"language":"English"}),
            input: json!({"type":"string"}),
            output: serde_json::to_value(schemars::schema_for!(crate::meaning::Reading))?,
        },
    )?)
}

fn searcher(seed: u64) -> Result<meden_runtime::plugin::Descriptor> {
    Ok(meden_runtime::plugin::Descriptor::new(
        meden_runtime::plugin::Definition {
            id: "fixture.search",
            role: meden_runtime::plugin::Role::CounterexampleSearch,
            version: "1",
            formats: vec!["fixture".into()],
            implementation: b"search recipe",
            configuration: json!({"seed":seed}),
            input: crate::search::input_schema::<Snapshot>()?,
            output: crate::search::output_schema::<Snapshot, crate::adapters::lean::search::Report>(
            )?,
        },
    )?)
}
#[test]
fn search_changes_reuse_compilation_but_never_relabel_a_retained_observation() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let mut raw = observed()?;
    let check = semantic(&mut raw)?.checker.take().context("check")?;
    assert!(crate::view::SourcePage::from_snapshot(&raw).is_err());
    let original = store.observe(raw.clone(), &definition()?)?;
    let id = original.compilation().clone();
    assert!(store.searched(&original, &searcher(0)?)?.is_none());
    let report = crate::adapters::lean::search::Report {
        checks: BTreeMap::from([("claim".into(), check)]),
    };
    let searched = store.retain_search(original, &searcher(0)?, report.clone())?;
    let initial_search = searched_id(&searched)?;
    let mut rendered = searched.snapshot().clone();
    store.render(searched, &renderer("reading")?, &mut rendered)?;
    let original = store.observe(raw.clone(), &definition()?)?;
    assert_eq!(&id, original.compilation());
    assert_eq!(store.index::<Artifact>()?.len(), 1);
    assert!(store.searched(&original, &searcher(1)?)?.is_none());
    let mut changed_report = report;
    let changed = changed_report.checks.get_mut("claim").context("claim")?;
    changed.seed = Some(1);
    changed.candidates = vec!["new witness".into()];
    let replacement = store.retain_search(original, &searcher(1)?, changed_report)?;
    assert_ne!(searched_id(&replacement)?, initial_search);
    assert_eq!(store.index::<Artifact>()?.len(), 1);
    assert_eq!(store.index::<Search>()?.len(), 2);
    let mut reading = store
        .rendered(
            replacement.compilation(),
            &replacement.snapshot,
            &renderer("reading")?,
        )?
        .context("independent reading reuse")?;
    assert_eq!(
        semantic(&mut reading)?
            .checker
            .as_ref()
            .context("new check")?
            .seed,
        Some(1)
    );
    assert_eq!(
        semantic(&mut reading)?
            .checker
            .as_ref()
            .context("new check")?
            .candidates,
        vec!["new witness"]
    );
    let original = store.observe(raw.clone(), &definition()?)?;
    let hit = store
        .searched(&original, &searcher(0)?)?
        .context("retained search")?;
    assert_eq!(searched_id(&hit)?, initial_search);
    let mut index = store.index::<Search>()?;
    index.insert(
        hash(&(&id, searcher(2)?.identity()))?,
        initial_search.clone(),
    );
    crate::storage::write(&store.0.join("searches-index.json"), &index)?;
    assert!(store.searched(&original, &searcher(2)?).is_err());
    let path = store
        .0
        .join("searches")
        .join(format!("{initial_search}.json"));
    let mut json: serde_json::Value = crate::storage::read(&path)?.context("search")?;
    *json
        .pointer_mut("/report/checks/claim/status")
        .context("status")? = json!("not_found");
    crate::storage::write(&path, &json)?;
    assert!(store.searched(&original, &searcher(0)?).is_err());
    assert!(semantic(&mut raw)?.checker.is_none());
    Ok(())
}

#[test]
fn compiler_and_renderer_definitions_partition_reuse_without_relabeling_old_data() -> Result<()> {
    let root = tempfile::tempdir()?;
    let store = Store(root.path().into());
    let first = definition()?;
    let second = meden_runtime::plugin::Descriptor::new(meden_runtime::plugin::Definition {
        id: "replacement.compiler",
        role: meden_runtime::plugin::Role::Compiler,
        version: "1",
        formats: vec!["fixture".into()],
        implementation: b"replacement compiler",
        configuration: json!({"seed":42}),
        input: crate::compiler::input_schema()?,
        output: crate::compiler::output_schema::<crate::adapters::lean::Snapshot>()?,
    })?;
    let snapshot = observed()?;
    let a = store.observe(snapshot.clone(), &first)?;
    let a_id = a.compilation().clone();
    let mut rendered = snapshot.clone();
    store.render(a, &renderer("first.renderer")?, &mut rendered)?;
    let b = store.observe(snapshot.clone(), &second)?;
    assert_ne!(a_id, *b.compilation());
    assert!(store
        .rendered(b.compilation(), &snapshot, &renderer("first.renderer")?)?
        .is_none());
    let artifact: Artifact = store.read(&a_id)?;
    artifact.check_compiler(&first)?;
    assert!(artifact.check_compiler(&second).is_err());
    assert!(artifact
        .check_compiler(&renderer("first.renderer")?)
        .is_err());
    assert_ne!(
        scope(&["Spec.lean".into()], false, &first)?,
        scope(&["Spec.lean".into()], false, &second)?
    );
    assert!(store
        .rendered(&a_id, &snapshot, &renderer("second.renderer")?)?
        .is_none());
    assert_eq!(
        store.read::<Artifact>(&a_id)?.compiler,
        serde_json::to_value(first)?
    );
    Ok(())
}

#[test]
fn a_searched_observation_cannot_be_reused_as_an_unsearched_parent() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let mut raw = observed()?;
    let check = semantic(&mut raw)?.checker.take().context("check")?;
    let original = store.observe(raw, &definition()?)?;
    assert!(original.search_id().is_none());
    let compilation = original.compilation().clone();
    assert_eq!(
        crate::search::Request::new(&original)?.observation(),
        &compilation
    );
    let report = crate::adapters::lean::search::Report {
        checks: BTreeMap::from([("claim".into(), check)]),
    };
    let searched = store.retain_search(original, &searcher(0)?, report.clone())?;
    assert_eq!(searched.compilation(), &compilation);
    assert!(searched.search_id().is_some());
    assert!(crate::search::Request::new(&searched).is_err());
    let before = std::fs::read(store.0.join("searches-index.json"))?;
    assert!(store.searched(&searched, &searcher(0)?).is_err());
    assert!(store
        .retain_search(searched, &searcher(1)?, report)
        .is_err());
    assert_eq!(before, std::fs::read(store.0.join("searches-index.json"))?);
    // Empty graphs have no checker field on which the dispatch adapter could
    // infer stage. The original owner must still forbid a second search parent.
    let mut empty = observed()?;
    empty
        .atlas
        .as_mut()
        .context("empty graph")?
        .graph
        .nodes
        .clear();
    let compiled = store.observe(empty, &definition()?)?;
    let searched = store.retain_search(
        compiled,
        &searcher(0)?,
        crate::adapters::lean::search::Report {
            checks: BTreeMap::new(),
        },
    )?;
    assert!(crate::search::Request::new(&searched).is_err());
    Ok(())
}

#[test]
fn a_decoded_foreign_identity_does_not_establish_artifact_kind_or_content() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let compilation = store
        .observe(observed()?, &definition()?)?
        .compilation()
        .clone();
    let original = Search {
        observation: compilation,
        searcher: serde_json::to_value(searcher(0)?)?,
        report: crate::adapters::lean::search::Report {
            checks: BTreeMap::new(),
        },
    };
    let id = store.write("original lookup".into(), &original)?;
    let decoded: Id<MeaningRendering> = serde_json::from_value(json!(id.as_str()))?;
    let foreign = store.0.join("renderings").join(format!("{decoded}.json"));
    std::fs::create_dir_all(foreign.parent().context("foreign parent")?)?;
    std::fs::copy(
        store.0.join("searches").join(format!("{id}.json")),
        &foreign,
    )?;
    // Even a complete, correctly checksummed foreign body must decode as its
    // requested role; a digest is never a permission or role assertion.
    assert!(store.read::<Rendering>(&decoded).is_err());
    assert_eq!(
        serde_json::to_value(store.read::<Search>(&id)?)?,
        serde_json::to_value(original)?
    );
    for invalid in [
        "",
        "../artifact",
        &"A".repeat(64),
        &"f".repeat(63),
        &format!("sha256:{}", "f".repeat(64)),
    ] {
        for role in ["observations", "searches", "renderings"] {
            crate::storage::write(
                &store.0.join(format!("{role}-index.json")),
                &json!({"original lookup":invalid}),
            )?;
        }
        assert!(store.index::<Artifact>().is_err());
        assert!(store.index::<Search>().is_err());
        assert!(store.index::<Rendering>().is_err());
    }
    assert_eq!(
        crate::search::input_schema::<Snapshot>()?.pointer("/properties/observation/pattern"),
        Some(&json!("^[a-f0-9]{64}$"))
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: proptest::test_runner::RngSeed::Fixed(20_261_125),
        ..ProptestConfig::default()
    })]
    #[test]
    #[allow(clippy::unwrap_used)] // Fixture/codec failures are shrinkable property failures.
    fn original_cache_parent_bytes_and_hash_framing_are_unchanged(
        chars in proptest::collection::vec(any::<char>(), 0..1024),
        digest in any::<[u8;32]>(),
    ) {
        #[derive(Serialize)]
        struct OriginalRendering<'a> { observation: &'a str, renderer: &'a serde_json::Value, readings: &'a [Reading] }
        #[derive(Serialize)]
        struct OriginalSearch<'a> { observation: &'a str, searcher: &'a serde_json::Value, report: &'a crate::adapters::lean::search::Report }
        let text: String = chars.into_iter().collect();
        let original = observed().unwrap();
        let artifact = Artifact { version: 2, compiler: serde_json::to_value(definition().unwrap()).unwrap(), snapshot: original };
        let parent = Id::of(&artifact).unwrap();
        let expected_parent = crate::atlas::digest(&serde_json::to_vec(&artifact).unwrap());
        prop_assert_eq!(parent.as_str(), &expected_parent);
        let rendering = Rendering { observation: parent.clone(), renderer: json!({"original":text}), readings:vec![Reading {node:"claim".into(), sentence:text, renderer:None, error:None}] };
        let original_rendering = OriginalRendering { observation:&expected_parent, renderer:&rendering.renderer, readings:&rendering.readings };
        let expected = serde_json::to_vec(&original_rendering).unwrap();
        prop_assert_eq!(&serde_json::to_vec(&rendering).unwrap(), &expected);
        let actual_id = Id::of(&rendering).unwrap();
        prop_assert_eq!(actual_id.as_str(), crate::atlas::digest(&expected));
        let search = Search { observation:parent, searcher:json!({"method":"original"}), report: crate::adapters::lean::search::Report {checks:BTreeMap::new()} };
        let original_search = OriginalSearch { observation:&expected_parent, searcher:&search.searcher, report:&search.report };
        let expected = serde_json::to_vec(&original_search).unwrap();
        prop_assert_eq!(&serde_json::to_vec(&search).unwrap(), &expected);
        let actual_id = Id::of(&search).unwrap();
        prop_assert_eq!(actual_id.as_str(), crate::atlas::digest(&expected));
        let mut spelling = String::new();
        for byte in digest { write!(spelling, "{byte:02x}").unwrap(); }
        let decoded: Id<Compilation> = serde_json::from_value(json!(spelling)).unwrap();
        prop_assert_eq!(serde_json::to_value(&decoded).unwrap(), json!(spelling));
        prop_assert_eq!(decoded.as_str(), spelling);
    }
}
