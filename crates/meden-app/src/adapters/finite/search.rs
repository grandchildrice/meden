//! Typed complete per-property observations from actual confined Z3 and the
//! independent finite evaluator. No checked artifact or receipt is retained.
use super::compiled::Compiled;
use crate::{adapters::Context, search};
use anyhow::{Context as _, Result};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    verification::finite,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    pub properties: BTreeMap<String, finite::Observation>,
}
impl search::Observation<Compiled> for Report {
    fn validate(&self, original: &Compiled) -> Result<()> {
        anyhow::ensure!(
            self.properties
                .keys()
                .collect::<std::collections::BTreeSet<_>>()
                == original
                    .contract
                    .properties
                    .iter()
                    .map(|p| &p.name)
                    .collect(),
            "Finite search omitted or changed original properties"
        );
        for (name, result) in &self.properties {
            if let finite::Observation::Counterexample { witness } = result {
                anyhow::ensure!(
                    &witness.property == name
                        && original
                            .contract
                            .validates_witness(&original.model, witness)?,
                    "Finite search witness does not refute its original property"
                );
            }
        }
        Ok(())
    }
}
pub(super) struct Searcher {
    pub worker: PathBuf,
}
impl search::Searcher for Searcher {
    type Input = Compiled;
    type Output = Report;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "finite.z3",
            role: Role::CounterexampleSearch,
            version: "1",
            formats: vec!["finite-contract-json-v1".into()],
            implementation: &[
                include_bytes!("search.rs").as_slice(),
                include_bytes!("../../search.rs").as_slice(),
            ]
            .concat(),
            configuration: serde_json::json!({"resources":finite::resource_identity(&self.worker)?,"method":finite::descriptor()?}),
            input: search::input_schema::<Compiled>()?,
            output: search::output_schema::<Compiled, Report>()?,
        })?)
    }
    fn search(
        &self,
        context: &Context<'_>,
        call: search::Invocation<Compiled, Report>,
    ) -> Result<search::Completed<Compiled, Report>> {
        let original = call.request().snapshot();
        let mut properties = BTreeMap::new();
        for property in &original.contract.properties {
            context.control.with_active(|| Ok(()))?;
            if let Some(observer) = &context.observer {
                observer(
                    "Searching finite counterexamples",
                    std::time::Duration::ZERO,
                    None,
                );
            }
            let mut contract = original.contract.clone();
            contract.properties = vec![property.clone()];
            let prepared = finite::prepare(
                original
                    .source
                    .files
                    .get(&contract.source_path)
                    .context("Finite source unavailable")?,
                &contract,
                &self.worker,
                context.control,
            )?;
            let observe = |progress: &meden_runtime::process::OutputProgress| {
                if let Some(observer) = &context.observer {
                    observer(
                        "Searching finite counterexamples",
                        std::time::Duration::ZERO,
                        Some(progress),
                    );
                }
            };
            properties.insert(
                property.name.clone(),
                prepared.observe(context.control, Some(&observe))?,
            );
        }
        Ok(call.observe(Report { properties }))
    }
}
