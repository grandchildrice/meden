//! Typed candidate-search roles over backend-owned compiled inputs and reports.
//! No search result admits evidence, authorizes I/O or supplies a universal AST.
use crate::{
    adapters::Context,
    compiler,
    explanation::cache::{
        artifact::{Compilation, Id},
        Observation as Compiled,
    },
};
use anyhow::{Context as _, Result};
use meden_runtime::plugin::{Descriptor, Role};
use serde::Serialize;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

#[derive(Serialize, schemars::JsonSchema)]
#[serde(bound(serialize = "I: Serialize"))]
pub struct Request<I> {
    observation: Id<Compilation>,
    #[serde(serialize_with = "serialize_original")]
    #[schemars(with = "I")]
    snapshot: Arc<I>,
}
fn serialize_original<I: Serialize, S: serde::Serializer>(
    value: &Arc<I>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.as_ref().serialize(serializer)
}
impl<I: compiler::Observation> Request<I> {
    /// Retain the original immutable owner without cloning its decoded AST.
    pub fn new(original: &Compiled<I>) -> Result<Self> {
        original.unsearched()?;
        Ok(Self {
            observation: original.compilation().clone(),
            snapshot: original.shared(),
        })
    }
    pub fn observation(&self) -> &Id<Compilation> {
        &self.observation
    }
    pub fn snapshot(&self) -> &I {
        &self.snapshot
    }
    fn identity(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// A backend-owned decoded report, checked against the original compiled data.
/// Validation is observational; it neither changes that AST nor admits evidence.
pub trait Observation<I: compiler::Observation>:
    Serialize + schemars::JsonSchema + Send + Sync
{
    fn validate(&self, original: &I) -> Result<()>;
}

/// Consuming call with its original backend types and frozen definition.
/// ```compile_fail,E0308
/// use meden_app::search::{Invocation, Completed, Observation};
/// fn wrong<I: meden_app::compiler::Observation,A: Observation<I>,B: Observation<I>>(
///     call: Invocation<I,A>, result: B
/// ) -> Completed<I,A> { call.observe(result) }
/// ```
pub struct Invocation<I, R> {
    report: PhantomData<fn() -> R>,
    request: Request<I>,
    definition: Descriptor,
}
impl<I: compiler::Observation, R: Observation<I>> Invocation<I, R> {
    pub fn request(&self) -> &Request<I> {
        &self.request
    }
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    pub fn observe(self, report: R) -> Completed<I, R> {
        Completed {
            request: self.request,
            definition: self.definition,
            report,
        }
    }
}
pub struct Completed<I, R> {
    request: Request<I>,
    definition: Descriptor,
    report: R,
}
impl<I, R> Completed<I, R> {
    pub fn into_report(self) -> R {
        self.report
    }
}
pub fn input_schema<I: compiler::Observation>() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Request<I>))?)
}
pub fn output_schema<I: compiler::Observation, R: Observation<I>>() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(R))?)
}
pub trait Searcher: Send + Sync {
    type Input: compiler::Observation;
    type Output: Observation<Self::Input>;
    fn descriptor(&self) -> Result<Descriptor>;
    fn search(
        &self,
        context: &Context<'_>,
        invocation: Invocation<Self::Input, Self::Output>,
    ) -> Result<Completed<Self::Input, Self::Output>>;
}
struct Installed<I, R> {
    searcher: Box<dyn Searcher<Input = I, Output = R>>,
    definition: Descriptor,
}
pub struct Registration<'a, I, R> {
    installed: &'a Installed<I, R>,
}
impl<I: compiler::Observation, R: Observation<I>> Registration<'_, I, R> {
    pub fn definition(&self) -> &Descriptor {
        &self.installed.definition
    }
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.installed.searcher.descriptor()? == self.installed.definition,
            "Search method changed after selection"
        );
        Ok(())
    }
    pub fn search(&self, context: &Context<'_>, request: Request<I>) -> Result<Completed<I, R>> {
        self.dispatch(context.control, request, |searcher, input| {
            searcher.search(context, input)
        })
    }
    fn dispatch(
        &self,
        control: &meden_runtime::process::Control,
        request: Request<I>,
        call: impl FnOnce(
            &dyn Searcher<Input = I, Output = R>,
            Invocation<I, R>,
        ) -> Result<Completed<I, R>>,
    ) -> Result<Completed<I, R>> {
        control.with_active(|| Ok(()))?;
        self.validate()?;
        {
            let _timing = meden_runtime::metrics::Span::start("search.validate_input");
            request.snapshot.sources().validate()?;
            request.snapshot.validate()?;
        }
        let original = {
            let _timing = meden_runtime::metrics::Span::start("search.freeze_input");
            request.identity()?
        };
        let output = call(
            self.installed.searcher.as_ref(),
            Invocation {
                report: PhantomData,
                request,
                definition: self.installed.definition.clone(),
            },
        )?;
        self.validate()?;
        control.with_active(|| Ok(()))?;
        {
            let _timing = meden_runtime::metrics::Span::start("search.validate_binding");
            anyhow::ensure!(
                output.definition == self.installed.definition
                    && output.request.identity()? == original,
                "Search returned another invocation's observation"
            );
        }
        {
            let _timing = meden_runtime::metrics::Span::start("search.validate_report");
            output.report.validate(output.request.snapshot())?;
        }
        Ok(output)
    }
}
pub struct Registry<I, R> {
    installed: BTreeMap<String, Installed<I, R>>,
}
impl<I, R> Default for Registry<I, R> {
    fn default() -> Self {
        Self {
            installed: BTreeMap::new(),
        }
    }
}
impl<I: compiler::Observation, R: Observation<I>> Registry<I, R> {
    pub fn register(
        &mut self,
        searcher: impl Searcher<Input = I, Output = R> + 'static,
    ) -> Result<()> {
        let definition = searcher.descriptor()?;
        let id = definition.id().to_owned();
        anyhow::ensure!(
            definition.role() == Role::CounterexampleSearch
                && definition.accepts_schema(&input_schema::<I>()?)?
                && definition.accepts_output_schema(&output_schema::<I, R>()?)?,
            "Search definition does not match its typed role"
        );
        anyhow::ensure!(
            self.installed.len() < 64 && !self.installed.contains_key(&id),
            "Duplicate or excessive search registration"
        );
        self.installed.insert(
            id,
            Installed {
                searcher: Box::new(searcher),
                definition,
            },
        );
        Ok(())
    }
    pub fn select(&self, id: &str) -> Result<Registration<'_, I, R>> {
        let selected = Registration {
            installed: self
                .installed
                .get(id)
                .context("Search implementation is not registered")?,
        };
        selected.validate()?;
        Ok(selected)
    }
}
#[cfg(test)]
mod conformance;
#[cfg(test)]
mod tests;
