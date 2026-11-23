use super::*;
use serde_json::json;

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
fn semantic(snapshot: &mut Snapshot) -> Result<&mut Explanation> {
    snapshot
        .atlas
        .as_mut()
        .and_then(|g| g.graph.nodes.first_mut())
        .and_then(|n| n.semantic.as_mut())
        .context("semantic")
}
fn revision(snapshot: &Snapshot) -> Result<&str> {
    Ok(&snapshot.atlas.as_ref().context("graph")?.revision)
}

#[test]
fn renderer_change_reuses_raw_observations_and_binds_a_distinct_complete_view() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let original = observed()?;
    let id = store.observe(original.clone(), &definition()?)?.id;
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
    let retained: Artifact = store.read("observations", &id)?;
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
    assert!(store.index("renderings")?.is_empty());
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
            observation: "original".into(),
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
    let id = store.observe(original.clone(), &definition()?)?.id;
    // Restart with only the successfully persisted compiler/search artifact.
    let restarted = Store(directory.path().into());
    let artifact: Artifact = restarted.read("observations", &id)?;
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
    let id = original.id.clone();
    store.render(original, &renderer("renderer")?, &mut rendered)?;
    let rendering_id = store
        .index("renderings")?
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
    let original: Artifact = store.read("observations", &id)?;
    original.check()?;
    Ok(())
}

#[test]
fn stale_observation_identity_cannot_select_another_rendering() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store(directory.path().into());
    let mut first = observed()?;
    let original = store.observe(first.clone(), &definition()?)?;
    let id = original.id.clone();
    store.render(original, &renderer("renderer")?, &mut first)?;
    let first_rendering = store
        .index("renderings")?
        .into_values()
        .next()
        .context("rendering id")?;
    let mut other = observed()?;
    other.files.insert(
        "Spec.lean".into(),
        "-- changed\ntheorem claim : False := by sorry\n".into(),
    );
    other.revision = format!("sha256:{}", hash(&other.files)?);
    let other_id = store.observe(other.clone(), &definition()?)?.id;
    assert_ne!(id, other_id);
    assert!(store
        .rendered(&other_id, &other, &renderer("renderer")?)?
        .is_none());
    let mut index = store.index("renderings")?;
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
            output: crate::compiler::output_schema()?,
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
        output: crate::compiler::output_schema()?,
    })?;
    let snapshot = observed()?;
    let a = store.observe(snapshot.clone(), &first)?;
    let a_id = a.id.clone();
    let mut rendered = snapshot.clone();
    store.render(a, &renderer("first.renderer")?, &mut rendered)?;
    let b = store.observe(snapshot.clone(), &second)?;
    assert_ne!(a_id, b.id);
    assert!(store
        .rendered(&b.id, &snapshot, &renderer("first.renderer")?)?
        .is_none());
    let artifact: Artifact = store.read("observations", &a_id)?;
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
        store.read::<Artifact>("observations", &a_id)?.compiler,
        serde_json::to_value(first)?
    );
    Ok(())
}
