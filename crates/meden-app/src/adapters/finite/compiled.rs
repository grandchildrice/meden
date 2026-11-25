//! Original finite data decoding. No solver, rendering, acceptance or evidence.
use crate::compiler::Observation as _;
use crate::{
    adapters::Context,
    analysis::sources::{self, Source},
    compiler,
};
use anyhow::{Context as _, Result};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    verification::finite as data,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Compiled {
    pub(super) source: sources::Snapshot,
    pub(super) path: String,
    pub(super) contract: data::Contract,
    pub(super) model: data::Model,
}
impl Source for Compiled {
    fn sources(&self) -> &sources::Snapshot {
        &self.source
    }
}
impl compiler::Observation for Compiled {
    fn validate(&self) -> Result<()> {
        self.source.validate()?;
        let original = data::Contract::parse(
            self.source
                .files
                .get(&self.path)
                .context("Finite contract source unavailable")?,
        )?;
        anyhow::ensure!(
            original == self.contract
                && self.source.files.len() == 2
                && self.path != original.source_path,
            "Finite compilation does not match its original inputs"
        );
        anyhow::ensure!(
            original.decode_model(
                self.source
                    .files
                    .get(&original.source_path)
                    .context("Finite model source unavailable")?
            )? == self.model,
            "Finite model differs from its original input"
        );
        Ok(())
    }
}
pub struct Compiler;
impl compiler::Compiler for Compiler {
    type Output = Compiled;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "finite.transition",
            role: Role::Compiler,
            version: "1",
            formats: vec!["finite-contract-json-v1".into()],
            implementation: &[
                include_bytes!("compiled.rs").as_slice(),
                include_bytes!("../../compiler.rs").as_slice(),
                include_bytes!("../../analysis/sources.rs").as_slice(),
                include_bytes!("../../identity/snapshots.rs").as_slice(),
            ]
            .concat(),
            configuration: serde_json::json!({"decoder":data::decoding_identity()}),
            input: compiler::input_schema()?,
            output: compiler::output_schema::<Compiled>()?,
        })?)
    }
    fn compile(
        &self,
        context: &Context<'_>,
        call: compiler::Invocation<Compiled>,
    ) -> Result<compiler::Compiled<Compiled>> {
        let _span = meden_runtime::metrics::Span::start("finite.decode_inputs");
        anyhow::ensure!(
            call.paths().len() == 1
                && call
                    .paths()
                    .first()
                    .is_some_and(|p| p.ends_with(".contract.json")),
            "Select one finite .contract.json file"
        );
        let first = sources::capture(context.workspace, context.control, call.paths())?;
        let path = first
            .files
            .keys()
            .next()
            .context("Finite contract path unavailable")?
            .clone();
        let contract = data::Contract::parse(
            first
                .files
                .get(&path)
                .context("Original finite contract unavailable")?,
        )?;
        anyhow::ensure!(
            path != contract.source_path,
            "A finite contract cannot be its own transition model"
        );
        let source = sources::capture(
            context.workspace,
            context.control,
            &[path.clone(), contract.source_path.clone()],
        )?;
        anyhow::ensure!(
            source.files.get(&path) == first.files.get(&path),
            "Finite contract changed during source capture"
        );
        let model = contract.decode_model(
            source
                .files
                .get(&contract.source_path)
                .context("Original finite model unavailable")?,
        )?;
        let snapshot = Compiled {
            source,
            path,
            contract,
            model,
        };
        if let Ok(Some(hit)) = compiled(context, &call, &snapshot) {
            return Ok(hit);
        }
        let _compile = meden_runtime::metrics::Span::start("finite.compile");
        Ok(call.observe(snapshot))
    }
}

pub(super) fn scope(path: &str, method: &Descriptor) -> Result<String> {
    crate::explanation::cache::hash(&("finite-transition", path, method.identity()))
}
pub(super) fn compiled(
    context: &Context<'_>,
    call: &compiler::Invocation<Compiled>,
    current: &Compiled,
) -> Result<Option<compiler::Compiled<Compiled>>> {
    let store = crate::explanation::cache::Store::host(context.workspace)?;
    let Some(id) = store
        .index::<crate::explanation::cache::Artifact<Compiled>>()?
        .get(&scope(&current.path, call.definition())?)
        .cloned()
    else {
        return Ok(None);
    };
    let saved = store.read::<crate::explanation::cache::Artifact<Compiled>>(&id)?;
    anyhow::ensure!(
        saved.version == 2 && serde_json::to_vec(&saved.snapshot)? == serde_json::to_vec(current)?,
        "Finite compiler cache input changed"
    );
    saved.snapshot.validate()?;
    Ok(Some(call.restore(saved.snapshot, &saved.compiler)?))
}
