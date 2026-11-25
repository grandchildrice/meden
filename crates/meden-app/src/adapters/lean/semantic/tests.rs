use super::*;
use crate::atlas::Extraction;
use crate::identity::snapshots::GraphRevision;
use crate::identity::snapshots::SourceRevision;
use crate::semantic::{Input, Prepared, Target};
use proptest::prelude::*;
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]
    #[test]
    fn typed_context_keeps_the_complete_original_scope_and_excludes_observations(
        kinds in proptest::collection::vec(any::<bool>(),1..25),
        edges in proptest::collection::vec((any::<u8>(),any::<u8>(),0u8..5),0..80),
        text in ".{0,80}",
    ) {
        let result = compare_context(&kinds,&edges,&text);
        prop_assert!(result.is_ok(),"{result:?}");
    }
}

fn compare_context(kinds: &[bool], edges: &[(u8, u8, u8)], text: &str) -> Result<()> {
    let nodes=kinds.iter().enumerate().map(|(i,theorem)|json!({"id":format!("n{i}"),"shortName":format!("n{i}"),"kind":if *theorem {"theorem"} else {"definition"},"filePath":"M.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":text,"value":Some("FORMAL_VALUE"),"axioms":["original axiom"],"standardAxioms":[],"externalDependencies":["Original.external"],"searchKind":"proposition","meta":{"isMainTheorem":i==0},"semantic":{"version":1,"origin":"elaborated","sentence":"GENERATED_PROSE_SENTINEL","refutation":"False","checker":{"engine":"fixture","status":"not_found","candidates":[]}}})).collect::<Vec<_>>();
    let edge_kinds = [
        "theorem_type_to_definition",
        "theorem_value_to_definition",
        "theorem_value_to_theorem",
        "definition_value_to_definition",
        "future_proof_edge",
    ];
    let edges=edges.iter().map(|(a,b,k)|Ok(json!({"source":format!("n{}",usize::from(*a).checked_rem(kinds.len()).context("empty node corpus")?),"target":format!("n{}",usize::from(*b).checked_rem(kinds.len()).context("empty node corpus")?),"kind":edge_kinds.get(usize::from(*k)).context("edge kind")?}))).collect::<Result<Vec<_>>>()?;
    let graph: Extraction = serde_json::from_value(
        json!({"revision":format!("sha256:{}","1".repeat(64)),"upstream":"exact original upstream","resources_sha256":"fixture","graph":{"metadata":{"projectName":"fixture","leanVersion":"original version"},"nodes":nodes,"edges":edges,"externalReferences":[]}}),
    )?;
    let target = Target {
        file: "M.lean".into(),
        line: 1,
        name: "n0".into(),
    };
    let old = super::oracle::original(&graph, &target)?;
    let new = serde_json::to_value(context(&graph, &target)?)?;
    anyhow::ensure!(new == old, "Complete original scope changed");
    anyhow::ensure!(
        !serde_json::to_string(&new)?.contains("GENERATED_PROSE_SENTINEL"),
        "Observational prose leaked"
    );
    Ok(())
}

#[test]
fn scoped_context_prunes_only_known_proof_edges_and_requires_the_exact_graph() -> Result<()> {
    let files = BTreeMap::from([("M.lean".into(), "source\n".into())]);
    let node = |id: &str, kind: &str| json!({"id":id,"shortName":id,"kind":kind,"filePath":"M.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"Nat","value":null,"axioms":[],"standardAxioms":[],"externalDependencies":[]});
    let graph = json!({"metadata":{"projectName":"scope fixture","leanVersion":"test"},"nodes":[node("T","theorem"),node("Proof","theorem"),node("New","definition")],"edges":[{"source":"T","target":"Proof","kind":"theorem_value_to_theorem"},{"source":"T","target":"New","kind":"future_theorem_value_to_definition"}],"externalReferences":[]});
    let extraction: Extraction = serde_json::from_value(
        json!({"revision":format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(&graph)?)),"upstream":"fixture","resources_sha256":"fixture","graph":graph}),
    )?;
    let target = Target {
        file: "M.lean".into(),
        line: 1,
        name: "T".into(),
    };
    let context = serde_json::to_value(context(&extraction, &target)?)?;
    let snapshot = Snapshot {
        source: crate::analysis::sources::Snapshot {
            revision: SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?)),
            files: files.clone(),
        },
        atlas: Some(extraction.clone()),
        collection: None,
    };
    let declarations = context
        .get("declarations")
        .and_then(Value::as_array)
        .context("scoped declarations")?;
    let names = declarations
        .iter()
        .filter_map(|node| node.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    assert_eq!(names, BTreeSet::from(["T", "New"]));
    let mut input = Input {
        source_revision: SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?)),
        graph_revision: Some(GraphRevision::parse(&format!("sha256:{}", "0".repeat(64)))?),
        target,
        requirement: "original requirement".into(),
    };
    assert!(Prepared::new(input.clone(), &snapshot).is_err());
    input.graph_revision = Some(extraction.revision.clone());
    assert!(Prepared::new(input.clone(), &snapshot).is_ok());
    input.target.name = "missing declaration".into();
    assert!(Prepared::new(input, &snapshot).is_err());
    Ok(())
}
