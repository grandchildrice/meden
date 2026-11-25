//! Bounded cached navigation, separate from compilation or full import closure.
use super::collection::File;
use anyhow::{Context, Result};
use std::collections::BTreeSet;

/// Original inventory membership is checked independently of backend resolution.
/// Follow the selected file's imports and import-only intermediates, at most
/// 32 files, in original import order. Load only complete saved syntax records.
pub fn read<S>(
    inventory: &[File<S>],
    path: &str,
    resolve: impl Fn(&str, &str) -> Option<String>,
    mut load: impl FnMut(&str) -> Result<Option<File<S>>>,
) -> Result<Vec<File<S>>> {
    anyhow::ensure!(
        inventory.len() <= 250_000
            && inventory
                .windows(2)
                .all(|pair| matches!(pair,[a,b] if a.path < b.path)),
        "Invalid navigation inventory order"
    );
    let contains = |path: &str| {
        inventory
            .binary_search_by(|f| f.path.as_str().cmp(path))
            .is_ok()
    };
    anyhow::ensure!(
        contains(path),
        "Source is outside the prepared map inventory"
    );
    let first = load(path)?.context("Project map cache is unavailable; refresh the workspace")?;
    anyhow::ensure!(first.path == path, "Foreign cached navigation source");
    let mut indexed = vec![first];
    let mut seen = BTreeSet::from([path.to_owned()]);
    let mut cursor = 0usize;
    while cursor < indexed.len() && indexed.len() < 32 {
        let file = indexed.get(cursor).context("Invalid navigation cursor")?;
        let mut neighbors = Vec::new();
        if cursor == 0 || file.declarations == 0 {
            for import in &file.imports {
                if let Some(path) = resolve(&file.path, import) {
                    anyhow::ensure!(
                        contains(&path),
                        "Resolved navigation is outside the original inventory"
                    );
                    if seen.insert(path.clone()) {
                        neighbors.push(path);
                    }
                    if neighbors.len() >= 32usize.saturating_sub(indexed.len()) {
                        break;
                    }
                }
            }
        }
        for path in neighbors {
            let file =
                load(&path)?.context("Project map cache is incomplete; refresh the workspace")?;
            anyhow::ensure!(file.path == path, "Foreign cached navigation source");
            indexed.push(file);
        }
        cursor = cursor.saturating_add(1);
    }
    Ok(indexed)
}
#[cfg(test)]
mod tests;
