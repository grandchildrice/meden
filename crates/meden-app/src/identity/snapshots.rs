//! Snapshot identities are distinct application observations, never authority.
//! Decoding validates spelling; original content/binding checks still apply.
//!
//! A graph revision cannot identify source inputs:
//! ```compile_fail
//! use meden_app::identity::snapshots::{GraphRevision, SourceRevision};
//! fn wrong(graph: GraphRevision) -> SourceRevision { graph }
//! ```
//! Prepared inputs cannot replace an elaborated graph:
//! ```compile_fail
//! use meden_app::identity::snapshots::{GraphRevision, PreparedRevision};
//! fn wrong(inputs: PreparedRevision) -> GraphRevision { inputs }
//! ```
//! A caller cannot construct an unchecked source revision:
//! ```compile_fail
//! use meden_app::identity::snapshots::SourceRevision;
//! let unchecked = SourceRevision("not observed".into());
//! ```
//! The matching backend owner retains the original identity:
//! ```
//! use meden_app::{analysis::sources::Snapshot, identity::snapshots::SourceRevision};
//! fn original(snapshot: &Snapshot) -> &SourceRevision { &snapshot.revision }
//! ```
use anyhow::Result;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{digest::Output, Sha256};
use std::borrow::Cow;

macro_rules! revision {
    ($name:ident, $prefix:literal, $pattern:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(String);
        impl $name {
            /// Decode exactly one canonical original spelling, without normalization.
            /// This establishes neither origin, retained content nor freshness.
            pub fn parse(value: &str) -> Result<Self> {
                let digest = value.strip_prefix($prefix).unwrap_or("");
                anyhow::ensure!(
                    digest.len() == 64
                        && digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    concat!("Invalid ", stringify!($name))
                );
                Ok(Self(value.into()))
            }
            /// Explicit transport/diagnostic spelling; not another context's ID.
            pub fn as_str(&self) -> &str { &self.0 }
            /// Observe the owner's original digest without changing its framing.
            pub fn from_digest(digest: Output<Sha256>) -> Self {
                Self(format!(concat!($prefix, "{:x}"), digest))
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
        }
        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> { stringify!($name).into() }
            fn inline_schema() -> bool { true }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                schemars::json_schema!({"type":"string","pattern":$pattern})
            }
        }
    }
}
revision!(
    SourceRevision,
    "sha256:",
    "^sha256:[a-f0-9]{64}$",
    "Digest of the exact complete observed source map."
);
revision!(
    GraphRevision,
    "sha256:",
    "^sha256:[a-f0-9]{64}$",
    "Revision of one original graph or its bound search/reading projection."
);
revision!(
    PreparedRevision,
    "",
    "^[a-f0-9]{64}$",
    "Original complete prepared-compiler input fingerprint, distinct from source text."
);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{analysis::sources::Snapshot, semantic, view::SourcePage};
    use proptest::prelude::*;
    use serde_json::json;
    use sha2::Digest;
    use std::collections::BTreeMap;

    #[test]
    fn malformed_ids_fail_at_the_source_graph_and_prepared_input_boundaries() -> Result<()> {
        let digest = "a".repeat(64);
        let text = format!("sha256:{digest}");
        for invalid in [
            String::new(),
            "sha256:old".into(),
            digest.clone(),
            text.to_uppercase(),
            format!(" {text}"),
            format!("{text}\n"),
            format!("sha256:{}", "０".repeat(64)),
            format!("sha256:{}", "b".repeat(63)),
        ] {
            assert!(serde_json::from_value::<SourceRevision>(json!(invalid)).is_err());
            assert!(serde_json::from_value::<GraphRevision>(json!(invalid)).is_err());
        }
        for invalid in [
            String::new(),
            text.clone(),
            digest.to_uppercase(),
            format!(" {digest}"),
            format!("{digest}\0"),
            "f".repeat(65),
            "g".repeat(64),
        ] {
            assert!(serde_json::from_value::<PreparedRevision>(json!(invalid)).is_err());
        }
        for invalid in [json!(null), json!(1), json!([]), json!({"revision": text})] {
            assert!(serde_json::from_value::<SourceRevision>(invalid.clone()).is_err());
            assert!(serde_json::from_value::<GraphRevision>(invalid.clone()).is_err());
            assert!(serde_json::from_value::<PreparedRevision>(invalid).is_err());
        }
        let source = json!({"revision":text,"files":{"Spec.rs":"value"}});
        let original: Snapshot = serde_json::from_value(source.clone())?;
        assert_eq!(serde_json::to_value(original)?, source);
        let mut bad = source;
        *bad.get_mut("revision").unwrap() = json!(digest);
        assert!(serde_json::from_value::<Snapshot>(bad).is_err());
        let schema = crate::compiler::output_schema::<crate::adapters::lean::Snapshot>()?;
        assert_eq!(
            schema.pointer("/properties/revision/pattern"),
            Some(&json!("^sha256:[a-f0-9]{64}$"))
        );
        Ok(())
    }

    proptest! {
        #[test]
        fn all_digest_bytes_round_trip_with_original_framing_and_distinct_roles(bytes in any::<[u8;32]>()) {
            use std::fmt::Write as _;
            let mut old = String::new();
            for byte in bytes { prop_assert!(write!(old,"{byte:02x}").is_ok(), "Original hexadecimal oracle"); }
            let output: Output<Sha256> = bytes.into();
            let source = SourceRevision::from_digest(output);
            let graph = GraphRevision::from_digest(output);
            let inputs = PreparedRevision::from_digest(output);
            let prefixed = format!("sha256:{old}");
            prop_assert_eq!(source.as_str(), &prefixed);
            prop_assert_eq!(graph.as_str(), &prefixed);
            prop_assert_eq!(inputs.as_str(), &old);
            prop_assert_eq!(serde_json::to_value(&source).unwrap(), json!(prefixed));
            prop_assert_eq!(serde_json::from_value::<SourceRevision>(json!(prefixed)).unwrap(), source);
            prop_assert_eq!(serde_json::from_value::<GraphRevision>(json!(prefixed)).unwrap(), graph);
            prop_assert_eq!(serde_json::from_value::<PreparedRevision>(json!(old)).unwrap(), inputs);
        }
        #[test]
        fn original_source_hash_wire_and_freshness_checks_survive_the_typed_projection(chars in proptest::collection::vec(any::<char>(),0..1024)) {
            let text: String = chars.into_iter().collect();
            let files = BTreeMap::from([("Spec.rs".to_owned(),text)]);
            let old = format!("sha256:{}", crate::atlas::digest(&serde_json::to_vec(&files).unwrap()));
            let revision = SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files).unwrap()));
            let snapshot = Snapshot {revision: revision.clone(), files: files.clone()};
            prop_assert_eq!(serde_json::to_value(&snapshot).unwrap(), json!({"revision":old,"files":files}));
            let mut page = SourcePage::from_snapshot(&snapshot).unwrap();
            prop_assert_eq!(page.revision.as_str(), &old);
            prop_assert_eq!(page.graph.as_ref().unwrap().revision.as_str(), &old);
            page.revision = SourceRevision::from_digest(Sha256::digest(b"another complete input"));
            prop_assert!(page.validate().is_err());
            let input = semantic::Input {source_revision:page.revision,graph_revision:None,target:semantic::Target{file:"Spec.rs".into(),line:1,name:"target".into()},requirement:"Original requirement".into()};
            prop_assert!(semantic::Prepared::new(input, &snapshot).is_err());
        }
    }
}
