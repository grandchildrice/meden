//! Trusted syntax-provider location policy, never an execution request.
use crate::navigation::collection::File;
use std::path::Path;

/// Lean's shallow conventional entry module. Other providers opt in separately.
pub fn entry(path: &str) -> bool {
    path.split('/').count() <= 2
        && Path::new(path)
            .file_name()
            .is_some_and(|n| n == "Main.lean")
}
/// Bind the original immutable inventory before resolving lexical import names.
/// Unknown or ambiguous locations remain unexpanded external boundaries.
pub fn resolver<S>(files: &[File<S>]) -> impl Fn(&str, &str) -> Option<String> + '_ {
    let mut root = files
        .first()
        .and_then(|f| f.path.rsplit_once('/'))
        .map_or("", |(dir, _)| dir)
        .to_owned();
    while !root.is_empty()
        && files
            .iter()
            .any(|f| !f.path.starts_with(&format!("{root}/")))
    {
        root = root.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    }
    move |source, module| {
        if Path::new(source).extension().is_none_or(|e| e != "lean") {
            return None;
        }
        let relative = format!("{}.lean", module.replace('.', "/"));
        for candidate in [format!("{root}/{relative}"), relative.clone()] {
            if files.binary_search_by(|f| f.path.cmp(&candidate)).is_ok() {
                return Some(candidate);
            }
        }
        let suffix = format!("/{relative}");
        let mut matches = files.iter().filter(|f| f.path.ends_with(&suffix));
        let found = matches.next()?;
        matches.next().is_none().then(|| found.path.clone())
    }
}
#[cfg(test)]
mod tests;
