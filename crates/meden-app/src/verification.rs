//! Typed registered verification roles. The existing kernel owns acceptance and
//! issued-job admission; these adapters only decode, prepare and check data.
use meden_runtime::{
    plugin::{Descriptor, Role},
    process::Control,
    verification::{
        artifact::{Checked, Class, Inputs},
        Failure,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io,
    path::Path,
    sync::{Arc, OnceLock},
};

mod finite;
mod refinement;
mod schema;
/// Default host preparation, preserving the existing Rust-to-Lean workflow.
pub const DEFAULT_BACKEND: &str = "lean.refinement.v1";

/// Read-only metadata for every verifier; no prover-specific viewer is required.
#[derive(Clone, Debug)]
pub struct Description {
    /// Reviewed contract title.
    pub name: String,
    /// Original source path selected by the contract.
    pub source: String,
    /// Declared source interface.
    pub interface: String,
    /// Exact formal properties rendered for review.
    pub properties: Vec<(String, String, String)>,
    /// Full backend-specific review prepared before acceptance.
    pub review: String,
    /// Logical strength; bounded evidence never becomes refinement evidence.
    pub class: Class,
    /// Explicit scope and excluded claims.
    pub scope: String,
}

/// One immutable selected check, consumed exactly once by the application.
pub trait PreparedCheck: Send {
    /// Complete prepared input, implementation and resource hashes.
    fn inputs(&self) -> &Inputs;
    /// Return only the backend's validated complete output.
    fn check(self: Box<Self>, control: &Control) -> Result<Checked, Failure>;
}
/// A host-registered backend owns its specific data schema and validation.
/// Model data cannot provide an executable, result, checker or authority flag.
pub trait Backend: Send + Sync + 'static {
    /// Decoded backend-owned specification data.
    type Specification: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;
    /// Stable host registration key.
    fn id(&self) -> &'static str;
    /// Schema for this backend's data format.
    fn schema(&self) -> io::Result<Value>;
    /// Complete original method/configuration/schema identity.
    fn descriptor(&self) -> io::Result<Descriptor>;
    /// Decode and validate bounded data, without effectful checker execution.
    fn decode(&self, text: &str) -> io::Result<Self::Specification>;
    /// Produce immutable viewer data from the original typed specification.
    fn describe(&self, specification: &Self::Specification) -> Description;
    /// Fixed recipe/schema/configuration identity required by the obligation.
    fn checker_id(&self, specification: &Self::Specification) -> io::Result<String>;
    /// Host-only prerequisite preparation, before inference starts.
    fn setup(&self) -> anyhow::Result<()>;
    /// Validate a draft before it may reach host review; no evidence is created.
    fn check_draft(
        &self,
        specification: &Self::Specification,
        worker: &Path,
        control: &Control,
    ) -> Result<(), Failure>;
    /// Freeze the original source and selected resources before ticket issue.
    fn prepare(
        &self,
        specification: &Self::Specification,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure>;
}

trait Decoded: Send + Sync {
    fn check_draft(&self, worker: &Path, control: &Control) -> Result<(), Failure>;
    fn prepare(
        &self,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure>;
}
struct Typed<B: Backend> {
    backend: Arc<B>,
    specification: B::Specification,
    descriptor: Descriptor,
}
impl<B: Backend> Typed<B> {
    fn current(&self) -> Result<(), Failure> {
        if self.backend.id() != self.descriptor.id()
            || self.backend.descriptor().map_err(Failure::unavailable)? != self.descriptor
            || !self
                .descriptor
                .accepts_schema(&self.backend.schema().map_err(Failure::unavailable)?)
                .map_err(Failure::unavailable)?
        {
            return Err(Failure::unavailable(
                "Registered verification method changed after decoding",
            ));
        }
        Ok(())
    }
}
impl<B: Backend> Decoded for Typed<B> {
    fn check_draft(&self, worker: &Path, control: &Control) -> Result<(), Failure> {
        self.current()?;
        self.backend
            .check_draft(&self.specification, worker, control)?;
        self.current()
    }
    fn prepare(
        &self,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure> {
        self.current()?;
        let prepared = self
            .backend
            .prepare(&self.specification, source, worker, control)?;
        self.current()?;
        let inputs = prepared.inputs().clone();
        if inputs.registration_sha256 != self.descriptor.identity() {
            return Err(Failure::unavailable(
                "Prepared verifier differs from its registered definition",
            ));
        }
        Ok(Box::new(RegisteredCheck {
            prepared,
            inputs,
            descriptor: self.descriptor.clone(),
        }))
    }
}

struct RegisteredCheck {
    prepared: Box<dyn PreparedCheck>,
    inputs: Inputs,
    descriptor: Descriptor,
}
impl PreparedCheck for RegisteredCheck {
    fn inputs(&self) -> &Inputs {
        &self.inputs
    }
    fn check(self: Box<Self>, control: &Control) -> Result<Checked, Failure> {
        let checked = self.prepared.check(control)?;
        if checked.report().registration != self.descriptor || checked.inputs() != self.inputs {
            return Err(Failure::unavailable(
                "Verifier output differs from the original registration or prepared inputs",
            ));
        }
        Ok(checked)
    }
}

/// Immutable decoded contract, canonical bytes and prepared viewer projection.
/// Only registered typed decoders construct it; serialization contains data only.
#[derive(Clone)]
pub struct Contract {
    id: &'static str,
    bytes: Arc<[u8]>,
    value: Value,
    description: Description,
    descriptor: Descriptor,
    checker: String,
    decoded: Arc<dyn Decoded>,
}
impl std::fmt::Debug for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Contract")
            .field("backend", &self.id)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}
impl PartialEq for Contract {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.bytes == other.bytes && self.descriptor == other.descriptor
    }
}
impl Eq for Contract {}
impl Serialize for Contract {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(s)
    }
}
impl<'de> Deserialize<'de> for Contract {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(d)?;
        Self::parse(&value.to_string()).map_err(serde::de::Error::custom)
    }
}
impl Contract {
    /// Parse against the installed host registrations. Drafts grant no authority.
    pub fn parse(text: &str) -> io::Result<Self> {
        Registry::host().parse(text)
    }
    /// The original canonical backend-owned data, independent of UI state.
    pub fn canonical(&self) -> io::Result<Vec<u8>> {
        Ok(self.bytes.to_vec())
    }
    /// Original contract content identity, not an acceptance token.
    pub fn identity(&self) -> io::Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!("{:x}", Sha256::digest(&self.bytes)))
    }
    /// Original backend registration.
    pub fn backend(&self) -> &str {
        self.id
    }
    /// Fixed obligation checker identity.
    pub fn checker_id(&self) -> io::Result<String> {
        Ok(self.checker.clone())
    }
    /// Frozen installed method; decoded metadata alone grants no authority.
    pub fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
    /// Generic immutable viewer projection.
    pub fn description(&self) -> &Description {
        &self.description
    }
    /// Reviewed title.
    pub fn name(&self) -> &str {
        &self.description.name
    }
    /// Original input file, used to derive exact read authority.
    pub fn source(&self) -> &str {
        &self.description.source
    }
    /// Original supported interface.
    pub fn api(&self) -> String {
        self.description.interface.clone()
    }
    /// Every original formal property, without an inferred proof flag.
    pub fn properties(&self) -> impl ExactSizeIterator<Item = (&str, &str, &str)> {
        self.description
            .properties
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
    }
    /// Full review and explicit evidence scope.
    pub fn review_text(&self) -> String {
        format!(
            "{}\n\nInstalled verification method: {} @ {}\nDefinition SHA-256: {}\nScope: {}\n",
            self.description.review,
            self.descriptor.id(),
            self.descriptor.version(),
            self.descriptor.identity(),
            self.description.scope
        )
    }
    /// Validate through the originally registered implementation.
    pub fn check_draft(&self, worker: &Path, control: &Control) -> Result<(), Failure> {
        self.decoded.check_draft(worker, control)
    }
    /// Prepare exactly the original typed contract and source.
    pub fn prepare(
        &self,
        source: &str,
        worker: &Path,
        control: &Control,
    ) -> Result<Box<dyn PreparedCheck>, Failure> {
        self.decoded.prepare(source, worker, control)
    }
}

/// Evidence-strength language shared by all presentation clients.
pub fn completed_label(class: Class) -> &'static str {
    match class {
        Class::Empirical => "CHECKED · empirical observations",
        Class::Bounded => "CHECKED · complete within finite bounds",
        Class::Theorem => "VERIFIED · theorem under stated assumptions",
        Class::Refinement => "VERIFIED · implementation refinement",
    }
}

struct Definition {
    descriptor: Descriptor,
    schema: Value,
}
trait Registered: Send + Sync {
    fn definition(&self) -> io::Result<&Definition>;
    fn decode(&self, text: &str) -> io::Result<Contract>;
    fn setup(&self) -> anyhow::Result<()>;
}
struct Registration<B: Backend> {
    id: &'static str,
    backend: Arc<B>,
    frozen: OnceLock<Result<Definition, String>>,
}
impl<B: Backend> Registration<B> {
    fn new(backend: B) -> Self {
        let id = backend.id();
        Self {
            id,
            backend: Arc::new(backend),
            frozen: OnceLock::new(),
        }
    }
}
impl<B: Backend> Registered for Registration<B> {
    fn definition(&self) -> io::Result<&Definition> {
        self.frozen
            .get_or_init(|| {
                let result = (|| {
                    let descriptor = self.backend.descriptor()?;
                    let schema = self.backend.schema()?;
                    if descriptor.role() != Role::Verifier
                        || descriptor.id() != self.id
                        || self.backend.id() != self.id
                        || !descriptor.accepts_schema(&schema)?
                    {
                        return Err(io::Error::other(
                            "Verifier registration/schema identity mismatch",
                        ));
                    }
                    Ok(Definition { descriptor, schema })
                })();
                result.map_err(|error: io::Error| error.to_string())
            })
            .as_ref()
            .map_err(|error| io::Error::other(error.clone()))
    }
    fn decode(&self, text: &str) -> io::Result<Contract> {
        let definition = self.definition()?;
        if self.backend.id() != self.id
            || self.backend.descriptor()? != definition.descriptor
            || !definition
                .descriptor
                .accepts_schema(&self.backend.schema()?)?
        {
            return Err(io::Error::other(
                "Verifier definition changed after registration",
            ));
        }
        let specification = self
            .backend
            .decode(text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let bytes = serde_json::to_vec(&specification)?;
        Ok(Contract {
            id: self.id,
            value: serde_json::from_slice(&bytes)?,
            bytes: bytes.into(),
            description: self.backend.describe(&specification),
            descriptor: definition.descriptor.clone(),
            checker: self.backend.checker_id(&specification)?,
            decoded: Arc::new(Typed {
                backend: Arc::clone(&self.backend),
                specification,
                descriptor: definition.descriptor.clone(),
            }),
        })
    }
    fn setup(&self) -> anyhow::Result<()> {
        self.definition()?;
        self.backend.setup()
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use meden_runtime::verification::{artifact::Report, finite::Contract as Specification};
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    struct Changing {
        phase: Arc<AtomicU8>,
        entered: Arc<AtomicBool>,
    }
    impl Backend for Changing {
        type Specification = Specification;
        fn id(&self) -> &'static str {
            if self.phase.load(Ordering::SeqCst) == 4 {
                "changed.host.id"
            } else {
                "z3.finite.v1"
            }
        }
        fn schema(&self) -> io::Result<Value> {
            if self.phase.load(Ordering::SeqCst) == 2 {
                Ok(serde_json::json!({"type":"null"}))
            } else {
                Specification::schema()
            }
        }
        fn descriptor(&self) -> io::Result<Descriptor> {
            let phase = self.phase.load(Ordering::SeqCst);
            if phase == 1 || phase == 3 {
                Descriptor::new(meden_runtime::plugin::Definition {
                    id: self.id(),
                    role: if phase == 3 {
                        Role::Model
                    } else {
                        Role::Verifier
                    },
                    version: "2",
                    formats: vec!["finite-bitvectors-v1".into()],
                    implementation: b"changed host method",
                    configuration: serde_json::json!({"seed":1}),
                    input: Specification::schema()?,
                    output: serde_json::to_value(schemars::schema_for!(Report))?,
                })
            } else {
                meden_runtime::verification::finite::descriptor()
            }
        }
        fn decode(&self, text: &str) -> io::Result<Specification> {
            Specification::parse(text)
        }
        fn describe(&self, s: &Specification) -> Description {
            finite::Finite.describe(s)
        }
        fn checker_id(&self, s: &Specification) -> io::Result<String> {
            s.checker_id()
        }
        fn setup(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn check_draft(&self, _: &Specification, _: &Path, _: &Control) -> Result<(), Failure> {
            self.entered.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn prepare(
            &self,
            _: &Specification,
            _: &str,
            _: &Path,
            _: &Control,
        ) -> Result<Box<dyn PreparedCheck>, Failure> {
            self.entered.store(true, Ordering::SeqCst);
            Err(Failure::unavailable(
                "No execution in a metadata-boundary fixture",
            ))
        }
    }
    #[test]
    fn changed_method_or_schema_cannot_enter_the_frozen_implementation() -> io::Result<()> {
        let phase = Arc::new(AtomicU8::new(0));
        let entered = Arc::new(AtomicBool::new(false));
        let mut registry = Registry::default();
        registry.register(Changing {
            phase: Arc::clone(&phase),
            entered: Arc::clone(&entered),
        })?;
        let original_schema = registry.parameters(None)?;
        let contract = registry.parse(include_str!(
            "../../../verification/examples/finite-counter.contract.json"
        ))?;
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        for changed in [1, 2, 4] {
            phase.store(changed, Ordering::SeqCst);
            assert_eq!(registry.parameters(None)?, original_schema);
            assert!(registry
                .parse(include_str!(
                    "../../../verification/examples/finite-counter.contract.json"
                ))
                .is_err());
            assert!(contract
                .check_draft(Path::new("/missing/worker"), &control)
                .is_err());
            assert!(contract
                .prepare("{}", Path::new("/missing/worker"), &control)
                .is_err());
            assert!(!entered.load(Ordering::SeqCst));
        }
        phase.store(3, Ordering::SeqCst);
        let mut fresh = Registry::default();
        assert!(fresh.register(Changing { phase, entered }).is_err());
        Ok(())
    }
}
/// Explicit host registrations. Unknown, duplicate or ambiguous formats fail.
#[derive(Default)]
pub struct Registry {
    registrations: BTreeMap<String, Box<dyn Registered>>,
}
impl Registry {
    /// Built-in host composition; no executable/path is read from model data.
    pub fn host() -> Self {
        Self {
            registrations: BTreeMap::from([
                (
                    DEFAULT_BACKEND.into(),
                    Box::new(Registration::new(refinement::Refinement {
                        bundle: crate::verifier::manifest(),
                    })) as Box<dyn Registered>,
                ),
                (
                    "z3.finite.v1".into(),
                    Box::new(Registration::new(finite::Finite)) as Box<dyn Registered>,
                ),
            ]),
        }
    }
    /// Add a distinct known host implementation, never through a model request.
    pub fn register<B: Backend>(&mut self, backend: B) -> io::Result<()> {
        let registration = Registration::new(backend);
        let id = registration.id;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
            || self.registrations.len() >= 64
            || self.registrations.contains_key(id)
        {
            return Err(io::Error::other(
                "Invalid or duplicate verification backend registration",
            ));
        }
        registration.definition()?;
        self.registrations.insert(id.into(), Box::new(registration));
        Ok(())
    }
    /// Check explicit host selection before opening a session.
    pub fn contains(&self, id: &str) -> bool {
        self.registrations.contains_key(id)
    }
    /// The selected host implementation prepares only its own prerequisites.
    pub fn setup(&self, id: &str) -> anyhow::Result<()> {
        self.registrations
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown verification backend {id}"))?
            .setup()
    }
    /// Decode one unambiguous bounded backend schema; preserve original values.
    pub fn parse(&self, text: &str) -> io::Result<Contract> {
        if text.len() > 65_536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Contract exceeds 64 KiB",
            ));
        }
        let mut decoded = None;
        let mut diagnostics = Vec::new();
        for (id, registration) in &self.registrations {
            match registration.decode(text) {
                Ok(value) => {
                    if decoded.is_some() {
                        return Err(io::Error::other(
                            "Ambiguous contract format; no backend selected",
                        ));
                    }
                    decoded = Some(value);
                }
                Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                    diagnostics.push(format!("{id}: {error}"));
                }
                Err(error) => return Err(error),
            }
        }
        decoded.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "No registered contract format accepted the proposal:\n{}",
                    diagnostics.join("\n")
                ),
            )
        })
    }
    /// Full function-argument schema. Each backend keeps its own namespace;
    /// local recursive references never collide with another backend's types.
    pub fn parameters(&self, selected: Option<&str>) -> io::Result<Value> {
        let mut variants = Vec::new();
        let mut definitions = serde_json::Map::new();
        for (id, registration) in &self.registrations {
            if selected.is_some_and(|selected| selected != id) {
                continue;
            }
            definitions.insert(
                id.clone(),
                schema::namespace(id, registration.definition()?.schema.clone())?,
            );
            variants.push(serde_json::json!({"$ref":format!("#/$defs/{id}")}));
        }
        if variants.is_empty() {
            return Err(io::Error::other("Unknown verification backend selection"));
        }
        let mut schema = serde_json::json!({"type":"object","properties":{"contract":{"oneOf":variants}},"required":["contract"],"additionalProperties":false});
        if !definitions.is_empty() {
            schema
                .as_object_mut()
                .ok_or_else(|| io::Error::other("Invalid function schema"))?
                .insert("$defs".into(), Value::Object(definitions));
        }
        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RUST: &str = include_str!("../../../verification/examples/counter.contract.json");
    const FINITE: &str =
        include_str!("../../../verification/examples/finite-counter.contract.json");

    #[test]
    fn registered_formats_preserve_the_original_canonical_data_and_strength() -> io::Result<()> {
        let registry = Registry::host();
        let rust = registry.parse(RUST)?;
        assert_eq!(
            rust.canonical()?,
            meden_runtime::verification::Contract::parse(RUST)?.canonical()?
        );
        assert_eq!(rust.description().class, Class::Refinement);
        let finite = registry.parse(FINITE)?;
        assert_eq!(finite.description().class, Class::Bounded);
        assert!(finite.review_text().contains("256 assignments"));
        assert_eq!(
            finite.canonical()?,
            meden_runtime::verification::finite::Contract::parse(FINITE)?.canonical()?
        );
        assert_eq!(
            finite,
            serde_json::from_value(serde_json::to_value(&finite)?)?
        );
        assert_ne!(finite.checker_id()?, rust.checker_id()?);
        Ok(())
    }

    #[test]
    fn malformed_ambiguous_and_duplicate_registrations_fail_closed() -> io::Result<()> {
        let mut registry = Registry::host();
        assert!(registry.register(finite::Finite).is_err());
        assert!(registry
            .parse(r#"{"format":"unregistered","passed":true}"#)
            .is_err());
        assert!(registry.parameters(Some("unknown")).is_err());
        let schema = registry.parameters(Some("z3.finite.v1"))?;
        assert_eq!(
            schema
                .pointer("/properties/contract/oneOf")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert!(schema.get("$defs").is_some());
        // Two genuine instances accepting the same data must remain ambiguous.
        registry.registrations.insert(
            "another-host-instance".into(),
            Box::new(Registration::new(finite::Finite)),
        );
        assert!(registry.parse(FINITE).is_err());
        Ok(())
    }
}
