//! Decoded compiler role. Results are observations; no cache state or plugin
//! registration accepts a contract, grants I/O or admits verification evidence.
use crate::{adapters::Context, analysis::sources::Snapshot};
use anyhow::Result;
use std::collections::BTreeMap;

/// An owned selection from an explicit application command.
pub struct Request {
    paths: Vec<String>,
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

/// Host-selected typed compiler implementation. The role never receives shell
/// commands, a model-selected executable or an evidence-admission callback.
pub trait Compiler: Send + Sync {
    /// Consume the original source request and return a complete decoded result.
    fn compile(&self, context: &Context<'_>, request: Request) -> Result<Snapshot>;
}

/// Immutable selected registration, distinct from model/workspace configuration.
pub struct Registration<'a> {
    id: &'a str,
    compiler: &'a dyn Compiler,
}
impl Registration<'_> {
    /// Stable host registration key; not a verification/checker identity.
    pub fn id(&self) -> &str {
        self.id
    }
    /// Dispatch the originally selected implementation without a replacement key.
    pub fn compile(self, context: &Context<'_>, request: Request) -> Result<Snapshot> {
        self.compiler.compile(context, request)
    }
}

/// Explicit host registry. It is never deserialized from repository/model data.
#[derive(Default)]
pub struct Registry {
    compilers: BTreeMap<String, Box<dyn Compiler>>,
}
impl Registry {
    /// Register once. Replacements and unbounded/malformed IDs are refused.
    pub fn register(&mut self, id: &str, compiler: impl Compiler + 'static) -> Result<()> {
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
        self.compilers.insert(id.into(), Box::new(compiler));
        Ok(())
    }
    /// Select only a known host implementation. Borrowing freezes registration
    /// against mutation while a selected operation is retained.
    ///
    /// ```compile_fail,E0505
    /// use meden_app::compiler::Registry;
    /// fn keep_original(registry: Registry) -> anyhow::Result<()> {
    ///     let selected = registry.select("installed.compiler")?;
    ///     drop(registry);
    ///     println!("{}", selected.id());
    ///     Ok(())
    /// }
    /// ```
    pub fn select(&self, id: &str) -> Result<Registration<'_>> {
        let (id, compiler) = self
            .compilers
            .get_key_value(id)
            .ok_or_else(|| anyhow::anyhow!("Compiler is not registered: {id}"))?;
        Ok(Registration {
            id,
            compiler: compiler.as_ref(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Unsupported;
    impl Compiler for Unsupported {
        fn compile(&self, _: &Context<'_>, _: Request) -> Result<Snapshot> {
            anyhow::bail!("unsupported fixture")
        }
    }

    #[test]
    fn registration_requires_a_known_unique_host_key() -> Result<()> {
        let mut registry = Registry::default();
        assert!(registry.select("not.installed").is_err());
        for invalid in ["", "../tool", "command --execute", "provider:unselected"] {
            assert!(registry.register(invalid, Unsupported).is_err());
        }
        registry.register("compiler.selected", Unsupported)?;
        assert!(registry.register("compiler.selected", Unsupported).is_err());
        assert_eq!(
            registry.select("compiler.selected")?.id(),
            "compiler.selected"
        );
        Ok(())
    }

    #[test]
    fn decoded_selection_is_bounded_and_keeps_exact_original_paths() -> Result<()> {
        let input = vec!["Nested/Source.lean".into()];
        assert_eq!(Request::new(input.clone())?.paths(), input);
        assert!(Request::new(Vec::new()).is_ok());
        assert!(Request::new(vec![String::new()]).is_err());
        assert!(Request::new(vec!["bad\0path".into()]).is_err());
        assert!(Request::new(vec!["x".repeat(4097)]).is_err());
        assert!(Request::new(vec!["Source.lean".into(); 33]).is_err());
        assert!(Request::new(vec!["Source.lean".into(); 2]).is_err());
        Ok(())
    }
}
