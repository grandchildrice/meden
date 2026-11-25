//! Read-only annotation port. Implementations supply already prepared descriptions;
//! this port cannot compile, search, admit evidence or restore model approval.
use crate::explanation::Explanation;
use anyhow::Result;
use std::collections::BTreeSet;

/// Description located in the original observed source, never a proof receipt.
#[derive(Clone)]
pub struct Annotation {
    /// Exact relative workspace source path.
    pub file: String,
    /// Original formal declaration name.
    pub name: String,
    /// One-based original source line.
    pub line: usize,
    /// Complete separately prepared description and search observation.
    pub semantic: Explanation,
    /// Backend-attributed target metadata, independent of proof strength.
    pub main_theorem: bool,
}
/// Host-selected read-only loader; resources and freshness remain its obligation.
pub type Loader<C> = fn(&C, &[String]) -> Result<Vec<Annotation>>;
struct Provider<C> {
    suffixes: Vec<String>,
    load: Loader<C>,
}
/// Trusted host composition. Paths select a registered reader, never an executable.
pub struct Registry<C> {
    providers: Vec<Provider<C>>,
}
impl<C> Default for Registry<C> {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
        }
    }
}
impl<C> Registry<C> {
    /// Refuse empty/overlapping suffixes before changing the registration set.
    pub fn register(&mut self, suffixes: &[&str], load: Loader<C>) -> Result<()> {
        let existing = self.providers.iter().flat_map(|p| &p.suffixes);
        let mut all: Vec<&str> = existing.map(String::as_str).collect();
        anyhow::ensure!(!suffixes.is_empty(), "Annotation provider has no formats");
        for suffix in suffixes {
            anyhow::ensure!(
                suffix.starts_with('.') && suffix.len() > 1 && !suffix.contains(['/', '\\', '\0']),
                "Invalid annotation format"
            );
            anyhow::ensure!(
                all.iter()
                    .all(|old| !old.ends_with(suffix) && !suffix.ends_with(old)),
                "Overlapping annotation formats"
            );
            all.push(suffix);
        }
        self.providers.push(Provider {
            suffixes: suffixes.iter().map(|s| (*s).to_owned()).collect(),
            load,
        });
        Ok(())
    }
    /// Read only selected formats and refuse a foreign/invalid result as a whole.
    /// Unsupported paths have no annotations; no fallback reader executes.
    pub fn load(&self, context: &C, paths: &[String]) -> Result<Vec<Annotation>> {
        let mut result = Vec::new();
        for provider in &self.providers {
            let selected: Vec<String> = paths
                .iter()
                .filter(|path| provider.suffixes.iter().any(|s| path.ends_with(s)))
                .cloned()
                .collect();
            if selected.is_empty() {
                continue;
            }
            let original: BTreeSet<_> = selected.iter().map(String::as_str).collect();
            let notes = (provider.load)(context, &selected)?;
            anyhow::ensure!(
                notes.len().saturating_add(result.len()) <= 1_048_576,
                "Too many navigation annotations"
            );
            anyhow::ensure!(
                notes.iter().all(|n| original.contains(n.file.as_str())
                    && !n.name.is_empty()
                    && n.name.len() <= 1_048_576
                    && (1..=1_048_577).contains(&n.line)
                    && n.semantic.valid()),
                "Invalid or foreign navigation annotation"
            );
            result.extend(notes);
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests;
