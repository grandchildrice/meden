//! Decoded compiler role. Results are observations; no cache state or plugin
//! registration accepts a contract, grants I/O or admits verification evidence.
use crate::{adapters::Context, analysis::sources};
use anyhow::Result;
use meden_runtime::plugin::{Descriptor, Role};
use std::collections::BTreeMap;
use std::marker::PhantomData;

/// An owned selection from an explicit application command.
#[derive(Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub struct Request {
    paths: Vec<String>,
}

/// Original request and definition, frozen by host registration before dispatch.
/// A result from another decoded backend type cannot replace this invocation:
/// ```compile_fail,E0308
/// use meden_app::compiler::{Compiled, Invocation, Observation};
/// fn wrong<A: Observation, B: Observation>(call: Invocation<A>, data: B) -> Compiled<A> {
///     call.observe(data)
/// }
/// ```
/// Matching decoded values retain their original type:
/// ```
/// use meden_app::compiler::{Compiled, Invocation, Observation};
/// fn matching<O: Observation>(call: Invocation<O>, data: O) -> Compiled<O> { call.observe(data) }
/// ```
/// Reading sources alone does not produce a compiler observation:
/// ```compile_fail
/// use meden_app::{analysis::sources::Snapshot, compiler::{Compiled, Invocation}};
/// fn wrong(call: Invocation<Snapshot>, raw: Snapshot) -> Compiled<Snapshot> { call.observe(raw) }
/// ```
pub struct Invocation<O> {
    observation: PhantomData<fn() -> O>,
    request: Request,
    definition: Descriptor,
}
impl<O: Observation> Invocation<O> {
    /// The original host source selection.
    pub fn paths(&self) -> &[String] {
        self.request.paths()
    }
    /// The immutable definition used for this dispatch and cache lookup.
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    /// Attribute newly decoded data to this original invocation. Cached results
    /// use the checked restoration path instead of being relabelled here.
    pub fn observe(self, snapshot: O) -> Compiled<O> {
        Compiled {
            reused: false,
            definition: self.definition,
            request: self.request,
            snapshot,
        }
    }
    pub(crate) fn restore(
        &self,
        snapshot: O,
        definition: &serde_json::Value,
    ) -> Result<Compiled<O>> {
        anyhow::ensure!(
            *definition == serde_json::to_value(&self.definition)?,
            "Cached compiler definition differs from its invocation"
        );
        Ok(Compiled {
            reused: true,
            definition: self.definition.clone(),
            request: self.request.clone(),
            snapshot,
        })
    }
}

/// Completed observation attributed only to the originally selected compiler.
/// Neither its binding nor its snapshot can be replaced by a consumer.
pub struct Compiled<O> {
    reused: bool,
    definition: Descriptor,
    request: Request,
    snapshot: O,
}
impl<O> Compiled<O> {
    /// Whether this original bound observation came through checked restoration.
    pub fn reused(&self) -> bool {
        self.reused
    }
    /// Consume the original method/output pair. These are observations, never
    /// an evidence receipt or permission to run another operation.
    pub fn into_parts(self) -> (Descriptor, O) {
        (self.definition, self.snapshot)
    }
}

/// Serialized source-selection shape of the compiler role.
pub fn input_schema() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Request))?)
}
/// Decoded observation shape; this cannot carry an evidence receipt.
pub fn output_schema<O: Observation>() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(O))?)
}
impl Request {
    /// Bound the request before dispatch. Empty selects configured project roots.
    /// Each backend still checks actual paths through the policy-bound worker.
    pub fn new(paths: Vec<String>) -> Result<Self> {
        anyhow::ensure!(
            paths.len() <= 32
                && paths
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == paths.len()
                && paths
                    .iter()
                    .all(|p| !p.is_empty() && p.len() <= 4096 && !p.contains('\0')),
            "Invalid compiler source selection"
        );
        Ok(Self { paths })
    }
    /// Original selection; implementations cannot substitute a later UI selection.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }
}

/// A decoded backend-owned result. The shared port checks original sources;
/// the backend owns its AST, completeness and compiler-only observation checks.
/// These checks do not admit evidence or establish physical I/O provenance.
pub trait Observation:
    sources::Source + serde::Serialize + schemars::JsonSchema + Send + Sync
{
    /// Validate this backend's complete decoded compiler observation.
    fn validate(&self) -> Result<()>;
}

/// Host-selected typed compiler implementation. The role never receives shell
/// commands, a model-selected executable or an evidence-admission callback.
pub trait Compiler: Send + Sync {
    /// Backend-owned decoded output; no universal prover AST is required.
    type Output: Observation;
    /// Actual implementation, configuration, resource and typed schema identity.
    fn descriptor(&self) -> Result<Descriptor>;
    /// Consume the original source request and return a complete decoded result.
    fn compile(
        &self,
        context: &Context<'_>,
        request: Invocation<Self::Output>,
    ) -> Result<Compiled<Self::Output>>;
}

struct Installed<O> {
    compiler: Box<dyn Compiler<Output = O>>,
    definition: Descriptor,
}
impl<O: Observation> Installed<O> {
    fn current(&self) -> Result<()> {
        anyhow::ensure!(
            self.compiler.descriptor()? == self.definition,
            "Compiler definition changed after registration"
        );
        Ok(())
    }
}

/// Immutable selected registration, distinct from model/workspace configuration.
pub struct Registration<'a, O> {
    id: &'a str,
    installed: &'a Installed<O>,
}
impl<O: Observation> Registration<'_, O> {
    /// Stable host registration key; not a verification/checker identity.
    pub fn id(&self) -> &str {
        self.id
    }
    /// Original definition frozen by host registration.
    pub fn definition(&self) -> &Descriptor {
        &self.installed.definition
    }
    /// Consume the original request through the retained host implementation.
    pub fn compile(self, context: &Context<'_>, request: Request) -> Result<Compiled<O>> {
        self.dispatch(request, |compiler, input| compiler.compile(context, input))
    }
    fn dispatch(
        self,
        request: Request,
        call: impl FnOnce(&dyn Compiler<Output = O>, Invocation<O>) -> Result<Compiled<O>>,
    ) -> Result<Compiled<O>> {
        self.installed.current()?;
        let selected = request
            .paths()
            .iter()
            .map(|p| crate::analysis::observe::relative("", p))
            .collect::<Result<Vec<_>>>()?;
        let definition = self.installed.definition.clone();
        let original = request.clone();
        let output = call(
            self.installed.compiler.as_ref(),
            Invocation {
                observation: PhantomData,
                request,
                definition: definition.clone(),
            },
        )?;
        self.installed.current()?;
        anyhow::ensure!(
            output.definition == definition && output.request == original,
            "Compiler returned an observation from another invocation"
        );
        let snapshot = &output.snapshot;
        snapshot.sources().validate()?;
        anyhow::ensure!(
            selected
                .iter()
                .all(|p| snapshot.sources().files.contains_key(p)),
            "Compiler observation omitted an originally selected source"
        );
        snapshot.validate()?;
        Ok(output)
    }
}

/// Explicit host registry. It is never deserialized from repository/model data.
pub struct Registry<O> {
    compilers: BTreeMap<String, Installed<O>>,
}
impl<O> Default for Registry<O> {
    fn default() -> Self {
        Self {
            compilers: BTreeMap::new(),
        }
    }
}
impl<O: Observation> Registry<O> {
    /// Register once. Replacements and unbounded/malformed IDs are refused.
    pub fn register(
        &mut self,
        id: &str,
        compiler: impl Compiler<Output = O> + 'static,
    ) -> Result<()> {
        anyhow::ensure!(
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')),
            "Invalid compiler registration ID"
        );
        anyhow::ensure!(
            !self.compilers.contains_key(id),
            "Duplicate compiler registration: {id}"
        );
        anyhow::ensure!(
            self.compilers.len() < 64,
            "Compiler registry exceeds 64 implementations"
        );
        let definition = compiler.descriptor()?;
        anyhow::ensure!(
            definition.role() == Role::Compiler
                && definition.id() == id
                && definition.accepts_schema(&input_schema()?)?
                && definition.accepts_output_schema(&output_schema::<O>()?)?,
            "Compiler definition does not match its typed role"
        );
        self.compilers.insert(
            id.into(),
            Installed {
                compiler: Box::new(compiler),
                definition,
            },
        );
        Ok(())
    }
    /// Select only a known host implementation. Borrowing freezes registration
    /// against mutation while a selected operation is retained.
    ///
    /// ```compile_fail,E0505
    /// use meden_app::compiler::Registry;
    /// fn keep_original<O: meden_app::compiler::Observation>(registry: Registry<O>) -> anyhow::Result<()> {
    ///     let selected = registry.select("installed.compiler")?;
    ///     drop(registry);
    ///     println!("{}", selected.id());
    ///     Ok(())
    /// }
    /// ```
    pub fn select(&self, id: &str) -> Result<Registration<'_, O>> {
        let (id, installed) = self
            .compilers
            .get_key_value(id)
            .ok_or_else(|| anyhow::anyhow!("Compiler is not registered: {id}"))?;
        installed.current()?;
        Ok(Registration { id, installed })
    }
}

#[cfg(test)]
mod tests;
