use super::*;
use proptest::prelude::*;
use serde_json::{json, Value};

fn summary() -> Result<project::Summary> {
    let root = tempfile::tempdir()?;
    let mut summary = project::Project::new(root.path())
        .expect("valid fixture workspace")
        .stored_status()?;
    let contract = crate::verification::Registry::host().parse_selected(
        include_str!("../../../../../verification/examples/finite-counter.contract.json"),
        Some("z3.finite.v1"),
    )?;
    summary.reference = Some(project::Reference {
        revision: RequirementRevision::new(u64::MAX)?,
        session: SessionId::parse("original host session")?,
        contract: ContractId::from_canonical(&contract.canonical()?),
    });
    summary.proposed = true;
    summary.source_path = contract.description().source.clone();
    summary.scope = contract.description().scope.clone();
    summary.evidence_class = Some(contract.description().class);
    summary.review = contract.review_text();
    summary.completion_label = crate::verification::completed_label(contract.description().class);
    summary.contract = Some(contract);
    Ok(summary)
}
fn accept(summary: &mut project::Summary) {
    summary.accepted = true;
    summary.accepted_reference = summary.reference.clone();
    summary.accepted_contract = summary.contract.clone();
}
fn artifact(summary: &mut project::Summary) {
    summary.artifact = Some("retained/original-output".into());
    summary.source_sha256 = Some("a".repeat(64));
    summary.artifact_sha256 = Some("b".repeat(64));
}
fn at(value: &Value, path: &str) -> Result<Value> {
    value
        .pointer(path)
        .cloned()
        .context("Expected original field")
}
fn published(project: &Project) -> Result<(ArtifactId<Artifact>, Arc<[u8]>, Value)> {
    let status = serde_json::to_value(project)?;
    let revision =
        serde_json::from_value(status.get("revision").context("Ready reference")?.clone())?;
    let bytes = project.bytes(&revision).context("exact body")?;
    let body = serde_json::from_slice(&bytes)?;
    Ok((revision, bytes, body))
}

proptest! {
    #[test]
    fn references_preserve_every_positive_u64(revision in 1u64..=u64::MAX) {
        let original = project::Reference { revision: RequirementRevision::new(revision).unwrap(), session: SessionId::parse("host session λ").unwrap(), contract: ContractId::parse(&"c".repeat(64)).unwrap() };
        let bytes = serde_json::to_vec(&Reference::new(&original)).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: Reference = serde_json::from_slice(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert!(decoded.original() == original);
        let json: Value = serde_json::from_slice(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decimal = revision.to_string();
        prop_assert_eq!(json.get("revision").ok_or_else(|| TestCaseError::fail("missing revision"))?.as_str(), Some(decimal.as_str()));
    }
}

#[test]
fn malformed_and_lossy_references_are_refused() -> Result<()> {
    for revision in [
        json!(0),
        json!(9_007_199_254_740_993_u64),
        json!("0"),
        json!("01"),
        json!("+1"),
        json!(" 1"),
        json!("１"),
        json!("18446744073709551616"),
    ] {
        assert!(serde_json::from_value::<Reference>(
            json!({"revision":revision,"session":"s","contract":"a".repeat(64)})
        )
        .is_err());
    }
    for revision in [1, 9_007_199_254_740_993, u64::MAX] {
        let original = project::Reference {
            revision: RequirementRevision::new(revision)?,
            session: SessionId::parse("s")?,
            contract: ContractId::parse(&"a".repeat(64))?,
        };
        assert!(Reference::new(&original).original() == original);
    }
    for (session, contract) in [
        ("", "a".repeat(64)),
        ("s", "A".repeat(64)),
        ("s", "a".repeat(63)),
    ] {
        assert!(serde_json::from_value::<Reference>(
            json!({"revision":"1","session":session,"contract":contract})
        )
        .is_err());
    }
    assert!(serde_json::from_value::<Reference>(
        json!({"revision":"1","session":"s","contract":"a".repeat(64),"grant":true})
    )
    .is_err());
    Ok(())
}

#[test]
fn complete_original_meaning_and_bounded_observations_remain_separate() -> Result<()> {
    // Synthetic Summary values test projection only; actual admission is covered
    // by the public finite-verifier TUI/browser journeys, not this fixture.
    let mut original = summary()?;
    let canonical = String::from_utf8(
        original
            .contract
            .as_ref()
            .context("contract")?
            .canonical()?,
    )?;
    let proposal = serde_json::to_value(Data::new(&original)?)?;
    assert_eq!(at(&proposal, "/state")?, "proposed");
    assert!(at(&proposal, "/accepted")?.is_null());
    assert_eq!(at(&proposal, "/review")?, original.review);
    assert_eq!(at(&proposal, "/contract/canonical")?, canonical);
    assert_eq!(
        at(&proposal, "/contract/reference/revision")?,
        u64::MAX.to_string()
    );
    assert_eq!(at(&proposal, "/contract/class")?, "bounded");
    accept(&mut original);
    let accepted = serde_json::to_value(Data::new(&original)?)?;
    assert_eq!(
        at(&accepted, "/evidence")?,
        json!({"state":"unverified","retained":null,"failure":null})
    );
    artifact(&mut original);
    original.verified = true;
    let mut view = Project::new(original.clone());
    let (old, bytes, current) = published(&view)?;
    assert_eq!(at(&current, "/data/evidence/state")?, "current");
    assert_eq!(at(&current, "/data/contract/class")?, "bounded");
    assert_eq!(
        at(&current, "/data/evidence/completion_label")?,
        original.completion_label
    );
    // Polls/identical observations reuse already frozen bytes.
    for _ in 0..100 {
        assert!(Arc::ptr_eq(&bytes, &view.bytes(&old).context("body")?));
    }
    view.observe(original);
    assert!(Arc::ptr_eq(&bytes, &view.bytes(&old).context("same body")?));
    view.invalidate();
    assert!(view.bytes(&old).is_none());
    let (new, _, stale) = published(&view)?;
    assert_ne!(old, new);
    assert_eq!(
        at(&stale, "/data/contract")?,
        at(&current, "/data/contract")?
    );
    assert_eq!(at(&stale, "/data/review")?, at(&current, "/data/review")?);
    assert_eq!(at(&stale, "/data/evidence/state")?, "unverified");
    assert_eq!(
        at(&stale, "/data/evidence/retained")?,
        at(&current, "/data/evidence/artifact")?
    );
    Ok(())
}

#[test]
fn replacement_retains_accepted_meaning_and_contradictions_retire_old_views() -> Result<()> {
    let mut original = summary()?;
    accept(&mut original);
    let mut view = Project::new(original.clone());
    let (old, _, _) = published(&view)?;
    original.accepted = false;
    original.reference.as_mut().context("proposal")?.revision =
        RequirementRevision::new(u64::MAX - 1)?;
    original.review = "Exact replacement review\n\"λ\"".into();
    view.observe(original.clone());
    let (_, _, proposal) = published(&view)?;
    assert_eq!(at(&proposal, "/data/state")?, "proposed");
    assert_eq!(
        at(&proposal, "/data/accepted/reference/revision")?,
        u64::MAX.to_string()
    );
    assert_eq!(at(&proposal, "/data/review")?, original.review);
    original.verified = true;
    view.observe(original);
    assert!(matches!(view.status, Status::Unavailable { .. }));
    assert!(view.bytes(&old).is_none());
    assert!(view.original.verified);
    for fault in 0..8 {
        let mut bad = summary()?;
        accept(&mut bad);
        match fault {
            0 => {
                bad.reference.as_mut().context("proposal")?.contract =
                    ContractId::parse(&"d".repeat(64))?
            }
            1 => bad.accepted_reference = None,
            2 => {
                bad.accepted_reference
                    .as_mut()
                    .context("accepted")?
                    .revision = RequirementRevision::new(1)?;
            }
            3 => bad.scope.push_str(" changed"),
            4 => bad.artifact = Some("incomplete".into()),
            5 => bad.verification_failure = Some(FailureKind::Unavailable),
            6 => {
                bad.verified = true;
                artifact(&mut bad);
                bad.verification_failure = Some(FailureKind::Unverified);
                bad.verification_diagnostic = Some("original failure".into());
            }
            _ => bad.reference = None,
        }
        assert!(Data::new(&bad).is_err(), "fault {fault}");
    }
    Ok(())
}

#[test]
fn no_contract_has_no_reference_and_cannot_publish_a_standalone_identity() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = project::Project::new(root.path())?;
    let mut empty = project.stored_status()?;
    assert!(empty.reference.is_none() && empty.accepted_reference.is_none());
    assert!(project.current_reference().is_err());
    assert_eq!(
        serde_json::to_value(Data::new(&empty)?)?,
        json!({"state":"empty"})
    );
    let mut view = Project::new(empty.clone());
    let (revision, _, _) = published(&view)?;
    empty.reference = summary()?.reference;
    view.observe(empty);
    assert!(matches!(view.status, Status::Unavailable { .. }));
    assert!(view.bytes(&revision).is_none());
    Ok(())
}

#[test]
fn oversized_escaped_reviews_keep_original_without_a_partial_or_old_body() -> Result<()> {
    let mut original = summary()?;
    let mut view = Project::new(original.clone());
    let (old, _, _) = published(&view)?;
    // Escaped bytes, rather than character count, determine envelope admission.
    original.review = "\0".repeat(super::super::publication::MAX_BYTES / 6);
    view.observe(original.clone());
    assert!(matches!(view.status, Status::Unavailable { .. }));
    assert!(view.bytes(&old).is_none());
    assert_eq!(view.original.review, original.review);
    Ok(())
}
