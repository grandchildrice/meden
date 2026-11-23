//! Immutable integration metadata. Identifiers and hashes are observations,
//! never capabilities, accepted contracts or evidence receipts.
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io;

/// Separate integration roles retain their own typed request/result ports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Source compilation and indexing.
    Compiler,
    /// Accepted-contract verification.
    Verifier,
    /// Deterministic meaning rendering.
    MeaningRenderer,
    /// Scoped candidate search, not evidence admission.
    CounterexampleSearch,
    /// Session-bound model inference.
    Model,
}

/// Host-owned definition inputs. No model or workspace decoder constructs a
/// registry from these values; role-specific implementations remain responsible
/// for actual syntax, resources, execution and output validation.
pub struct Definition<'a> {
    /// Stable host registration key.
    pub id: &'a str,
    /// Integration role.
    pub role: Role,
    /// Explicit interface/implementation version.
    pub version: &'a str,
    /// Supported format identifiers.
    pub formats: Vec<String>,
    /// Actual implementation/recipe bytes selected by the host.
    pub implementation: &'a [u8],
    /// Complete non-secret method configuration, including limits and seed.
    pub configuration: Value,
    /// Exact original input schema, before presentation namespacing.
    pub input: Value,
    /// Exact successful-result schema; its values cannot construct a receipt.
    pub output: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Metadata {
    id: String,
    role: Role,
    version: String,
    formats: Vec<String>,
    implementation_sha256: String,
    configuration_sha256: String,
    input_schema_sha256: String,
    output_schema_sha256: String,
}

/// Frozen, bounded method identity. Resource/worker observations are bound
/// separately at preparation, when those actual resources have been selected.
/// It is serializable for provenance but cannot be deserialized into authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct Descriptor {
    #[serde(flatten)]
    metadata: Metadata,
    #[serde(skip)]
    identity: String,
}
impl Descriptor {
    /// Validate and freeze the complete definition once.
    pub fn new(definition: Definition<'_>) -> io::Result<Self> {
        if !identifier(definition.id)
            || !identifier(definition.version)
            || definition.formats.is_empty()
            || definition.formats.len() > 64
            || definition.formats.iter().any(|f| !identifier(f))
            || definition.implementation.is_empty()
        {
            return Err(io::Error::other("Invalid plugin definition"));
        }
        let mut formats = definition.formats;
        formats.sort();
        if formats.windows(2).any(|pair| pair.first() == pair.get(1)) {
            return Err(io::Error::other("Duplicate plugin format"));
        }
        let metadata = Metadata {
            id: definition.id.into(),
            role: definition.role,
            version: definition.version.into(),
            formats,
            implementation_sha256: hash(definition.implementation),
            configuration_sha256: fingerprint(&definition.configuration)?,
            input_schema_sha256: fingerprint(&definition.input)?,
            output_schema_sha256: fingerprint(&definition.output)?,
        };
        let identity = fingerprint(&serde_json::to_value(&metadata)?)?;
        Ok(Self { metadata, identity })
    }
    /// Selected registration key, not an executable address.
    pub fn id(&self) -> &str {
        &self.metadata.id
    }
    /// Original role.
    pub fn role(&self) -> Role {
        self.metadata.role
    }
    /// Original declared version.
    pub fn version(&self) -> &str {
        &self.metadata.version
    }
    /// Canonical complete definition digest, not an authorization token.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Check the advertised schema against the original frozen definition.
    pub fn accepts_schema(&self, schema: &Value) -> io::Result<bool> {
        Ok(fingerprint(schema)? == self.metadata.input_schema_sha256)
    }
    /// Check the successful-result schema of the typed role port.
    pub fn accepts_output_schema(&self, schema: &Value) -> io::Result<bool> {
        Ok(fingerprint(schema)? == self.metadata.output_schema_sha256)
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Stable JSON identity independent of object insertion order. It preserves
/// array order and exact represented values and refuses oversized/deep metadata.
pub fn fingerprint(value: &Value) -> io::Result<String> {
    fn ordered(value: &Value, depth: usize, nodes: &mut usize) -> io::Result<Value> {
        *nodes = nodes.saturating_add(1);
        if depth > 64 || *nodes > 32_768 {
            return Err(io::Error::other(
                "Plugin metadata exceeds depth/node bounds",
            ));
        }
        Ok(match value {
            Value::Object(object) => {
                let mut object = object
                    .iter()
                    .map(|(key, value)| {
                        Ok((key.clone(), ordered(value, depth.saturating_add(1), nodes)?))
                    })
                    .collect::<io::Result<serde_json::Map<_, _>>>()?;
                object.sort_keys();
                Value::Object(object)
            }
            Value::Array(array) => Value::Array(
                array
                    .iter()
                    .map(|v| ordered(v, depth.saturating_add(1), nodes))
                    .collect::<io::Result<_>>()?,
            ),
            other => other.clone(),
        })
    }
    let bytes = serde_json::to_vec(&ordered(value, 0, &mut 0)?)?;
    if bytes.len() > 262_144 {
        return Err(io::Error::other("Plugin metadata exceeds 256 KiB"));
    }
    Ok(hash(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition() -> Definition<'static> {
        Definition {
            id: "finite",
            role: Role::Verifier,
            version: "1",
            formats: vec!["finite-v1".into()],
            implementation: b"original recipe",
            configuration: serde_json::json!({"seed":0,"timeout":15}),
            input: serde_json::json!({"type":"object"}),
            output: serde_json::json!({"type":"object"}),
        }
    }
    #[test]
    fn canonical_metadata_binds_every_original_method_parameter() -> io::Result<()> {
        assert_eq!(
            fingerprint(&serde_json::from_str(r#"{"b":{"y":2,"x":1},"a":0}"#)?)?,
            fingerprint(&serde_json::from_str(r#"{"a":0,"b":{"x":1,"y":2}}"#)?)?
        );
        let original = Descriptor::new(definition())?;
        for changed in [
            Definition {
                implementation: b"changed recipe",
                ..definition()
            },
            Definition {
                version: "2",
                ..definition()
            },
            Definition {
                configuration: serde_json::json!({"seed":1,"timeout":15}),
                ..definition()
            },
            Definition {
                input: serde_json::json!({"type":"string"}),
                ..definition()
            },
            Definition {
                output: serde_json::json!({"type":"string"}),
                ..definition()
            },
            Definition {
                formats: vec!["another-v1".into()],
                ..definition()
            },
        ] {
            assert_ne!(original.identity(), Descriptor::new(changed)?.identity());
        }
        assert!(Descriptor::new(Definition {
            id: "../command",
            ..definition()
        })
        .is_err());
        assert!(Descriptor::new(Definition {
            formats: vec!["same".into(), "same".into()],
            ..definition()
        })
        .is_err());
        assert!(original.accepts_schema(&definition().input)?);
        assert!(!original.accepts_schema(&serde_json::json!({"type":"string"}))?);
        assert!(original.accepts_output_schema(&definition().output)?);
        assert!(!original.accepts_output_schema(&serde_json::json!({"type":"string"}))?);
        Ok(())
    }
}
