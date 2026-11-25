//! Trusted-host pure navigation composition. Syntax indices neither select an
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
