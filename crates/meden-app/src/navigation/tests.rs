use super::*;
use anyhow::Context as _;
struct Host {
    marker: String,
    fault: u8,
}
use serde_json::json;

fn note(file: &str, name: &str, line: usize) -> Result<Annotation> {
    Ok(Annotation {
        file: file.into(),
        name: name.into(),
        line,
        main_theorem: true,
        semantic: serde_json::from_value(json!({
            "version":1,"origin":"elaborated","sentence":"Prepared description",
            "refutation":"Candidate only", "checker":{
                "engine":"fixture","status":"not_found","candidates":[]
            }
        }))?,
    })
}
#[test]
fn formats_dispatch_original_context_and_exact_paths_without_fallback() -> Result<()> {
    fn first(context: &Host, paths: &[String]) -> Result<Vec<Annotation>> {
        assert_eq!(context.marker, "host");
        assert_eq!(paths, &["a.lean", "nested/b.lean"]);
        Ok(vec![note("a.lean", "claim", 2)?])
    }
    fn second(context: &Host, paths: &[String]) -> Result<Vec<Annotation>> {
        assert_eq!(context.marker, "host");
        assert_eq!(paths, &["model.contract.json"]);
        Ok(vec![note("model.contract.json", "property.safe", 5)?])
    }
    let mut registry = Registry::default();
    registry.register(&[".lean"], first)?;
    registry.register(&[".contract.json", ".model.json"], second)?;
    let paths = [
        "a.lean",
        "unknown.rs",
        "model.contract.json",
        "nested/b.lean",
    ]
    .map(str::to_owned);
    let notes = registry.load(
        &Host {
            marker: "host".into(),
            fault: 0,
        },
        &paths,
    )?;
    assert_eq!(notes.len(), 2);
    assert_eq!(
        notes.iter().map(|n| n.file.as_str()).collect::<Vec<_>>(),
        ["a.lean", "model.contract.json"]
    );
    assert!(registry
        .load(
            &Host {
                marker: String::new(),
                fault: 0
            },
            &["unregistered.rs".into()]
        )?
        .is_empty());
    Ok(())
}
#[test]
fn overlapping_registration_is_atomic() -> Result<()> {
    fn forbidden(_: &Host, _: &[String]) -> Result<Vec<Annotation>> {
        anyhow::bail!("no selected format")
    }
    let mut registry = Registry::default();
    registry.register(&[".lean"], forbidden)?;
    for suffixes in [
        &[][..],
        &["lean"],
        &[".rs", ".lean"],
        &[".json", ".model.json"],
        &[".bad/path"],
        &[".bad\\path"],
        &[".bad\0"],
    ] {
        assert!(registry.register(suffixes, forbidden).is_err());
    }
    assert!(registry
        .load(
            &Host {
                marker: String::new(),
                fault: 0
            },
            &["unselected.rs".into(), "unselected.json".into()]
        )?
        .is_empty());
    assert_eq!(registry.providers.len(), 1);
    Ok(())
}
#[test]
fn a_foreign_or_invalid_result_cannot_return_a_successful_prefix() -> Result<()> {
    fn invalid(context: &Host, paths: &[String]) -> Result<Vec<Annotation>> {
        let path = paths.first().context("Missing selected input")?;
        let mut bad = note(path, "bad", 1)?;
        match context.fault {
            0 => bad.file = "foreign.lean".into(),
            1 => bad.name.clear(),
            2 => bad.line = 0,
            3 => bad.line = 1_048_578,
            4 => bad.semantic.checker.status = "proved".into(),
            _ => anyhow::bail!("Reader unavailable"),
        }
        Ok(vec![note(path, "valid", 1)?, bad])
    }
    let mut registry = Registry::default();
    registry.register(&[".lean"], invalid)?;
    for case in 0..=5 {
        assert!(registry
            .load(
                &Host {
                    marker: String::new(),
                    fault: case
                },
                &["selected.lean".into()]
            )
            .is_err());
    }
    Ok(())
}
