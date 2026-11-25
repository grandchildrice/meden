//! Trusted-host navigation composition. Syntax indices neither select an
//! executable nor imply compiled meaning, a completed search or proof evidence.
use super::source::{self, Symbol};
use std::path::Path;
pub fn supports(path: &str) -> bool {
    source::supports(Path::new(path)) || super::finite::navigation::supports(path)
}
/// Additional provider namespace; the unchanged default lexical provider keeps
/// its original namespace. New providers own their independent syntax identity.
pub fn additional_identity(path: &str) -> Option<String> {
    super::finite::navigation::supports(path).then(super::finite::navigation::identity)
}
pub fn index(path: &str, text: &str) -> (usize, usize, Vec<String>, Vec<Symbol>) {
    if super::finite::navigation::supports(path) {
        let symbols = super::finite::navigation::symbols(path, text).unwrap_or_default();
        let theorems = symbols.iter().filter(|s| s.kind == "theorem").count();
        return (symbols.len(), theorems, vec![], symbols);
    }
    let lean = Path::new(path).extension().is_some_and(|e| e == "lean");
    let (count, theorems, imports) = source::headers(text, lean);
    (count, theorems, imports, source::symbols(text, lean))
}

fn lean_annotations(
    context: &super::Context<'_>,
    paths: &[String],
) -> anyhow::Result<Vec<crate::navigation::Annotation>> {
    Ok(
        super::lean::cache::load(context.workspace, context.control, paths)?
            .into_iter()
            .map(|note| crate::navigation::Annotation {
                file: note.file,
                name: note.name,
                line: note.line,
                semantic: note.semantic,
                main_theorem: note.main_theorem,
            })
            .collect(),
    )
}
/// Host registration of prepared annotation readers, independent of GUI/TUI.
fn annotations<'a>() -> anyhow::Result<crate::navigation::Registry<super::Context<'a>>> {
    let mut registry = crate::navigation::Registry::default();
    registry.register(&[".lean"], lean_annotations)?;
    Ok(registry)
}
/// Overlay matching original locations only. Index lookup replaces repeated
/// complete-file and declaration scans; a failed load changes no symbol.
pub fn annotate<'a>(
    context: &super::Context<'_>,
    files: impl Iterator<Item = (&'a str, &'a mut [Symbol])>,
) -> anyhow::Result<()> {
    let _timing = meden_runtime::metrics::Span::start("navigation.annotations");
    let files: Vec<_> = files.collect();
    let paths = files
        .iter()
        .map(|(p, _)| (*p).to_owned())
        .collect::<Vec<_>>();
    let notes = {
        let _load = meden_runtime::metrics::Span::start("navigation.annotations.load");
        annotations()?.load(context, &paths)?
    };
    let _overlay = meden_runtime::metrics::Span::start("navigation.annotations.overlay");
    overlay(files, &notes);
    Ok(())
}
fn overlay<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a mut [Symbol])>,
    notes: &[crate::navigation::Annotation],
) {
    use std::collections::BTreeMap;
    let mut by_file: BTreeMap<&str, BTreeMap<(&str, usize), _>> = BTreeMap::new();
    for note in notes {
        by_file
            .entry(&note.file)
            .or_default()
            .insert((&note.name, note.line), note);
    }
    for (path, symbols) in files {
        let Some(notes) = by_file.get_mut(path) else {
            continue;
        };
        for symbol in symbols {
            if let Some(note) = notes.remove(&(symbol.name.as_str(), symbol.line)) {
                symbol.semantic = Some(note.semantic.clone());
                symbol.main_theorem = note.main_theorem;
            }
        }
    }
}
#[cfg(test)]
mod tests;
