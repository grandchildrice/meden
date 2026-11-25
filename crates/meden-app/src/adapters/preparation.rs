//! Native host composition. Registration/format selection stays outside consumers,
//! role ports and the authority kernel.
use crate::adapters::Context;
use crate::analysis::{
    prepared::{Backend, Preparation, Snapshot},
    sources,
};
use anyhow::Result;

struct Plain;
impl Backend for Plain {
    type Output = sources::Snapshot;
    fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Self::Output, bool)> {
        Ok((
            sources::capture(context.workspace, context.control, paths)?,
            false,
        ))
    }
}
struct Lean {
    lake: bool,
}
impl Backend for Lean {
    type Output = super::lean::Snapshot;
    fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Self::Output, bool)> {
        anyhow::ensure!(
            self.lake == super::lake::enabled()?,
            "Originally selected Lean preparation mode changed"
        );
        super::lean::pipeline::extract_cached(
            context.workspace,
            context.control,
            paths,
            context.observer.clone(),
        )
    }
    fn saved(
        &self,
        context: &Context<'_>,
        paths: &[String],
    ) -> Result<Option<crate::view::sources::SavedSource>> {
        super::lean::saved::load(context, paths)
    }
    fn background(&self) -> bool {
        !self.lake
    }
    fn details(&self, output: &Self::Output) -> Vec<String> {
        let collection = output.collection();
        let mut rows = vec![
            format!("Selected roots: {}", collection.roots.join(", ")),
            format!(
                "External modules (not expanded): {}",
                collection.external_modules.join(", ")
            ),
        ];
        rows.extend(
            collection
                .packages
                .iter()
                .map(|p| format!("Lake package: {} · {}", p.name, p.source_roots.join(", "))),
        );
        rows
    }
}

pub fn plain(context: &Context<'_>, paths: &[String]) -> Result<Snapshot> {
    Preparation::new(Plain)
        .read(context, paths)
        .map(|(snapshot, _)| snapshot)
}
pub fn lean() -> Result<Preparation> {
    Ok(Preparation::new(Lean {
        lake: super::lake::enabled()?,
    }))
}
fn registry() -> Result<crate::analysis::prepared::Registry> {
    let mut registry = crate::analysis::prepared::Registry::default();
    registry.register("lean.semantic", &[".lean"], lean)?;
    registry.register("finite.transition", &[".contract.json"], || {
        Ok(Preparation::new(super::finite::Backend::new(
            std::env::current_exe()?,
        )))
    })?;
    Ok(registry)
}
/// Pure format availability. Resource discovery occurs only after selection.
pub fn supports(path: &str) -> bool {
    registry().is_ok_and(|r| r.supports(path))
}
/// Default native file registration, chosen only from trusted host implementations.
pub fn for_path(path: &str) -> Result<Option<Preparation>> {
    registry()?.select(path)
}
pub fn read(context: &Context<'_>, paths: &[String], compiled: bool) -> Result<Snapshot> {
    if !compiled {
        return plain(context, paths);
    }
    let selected = if let Some(path) = paths.first() {
        for_path(path)?
            .ok_or_else(|| anyhow::anyhow!("No graph preparation is registered for {path}"))?
    } else {
        lean()?
    };
    selected.read(context, paths).map(|(snapshot, _)| snapshot)
}
