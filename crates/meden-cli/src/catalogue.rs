//! The invariant catalogue: what a property id actually means.
//!
//! An attestation says "this artifact has property `inv.v0.x`, proved by this
//! verifier". Nothing in that sentence says what `inv.v0.x` *is*. Without an
//! answer, a verifier could prove `True` and call it anything, and the gate
//! would accept it: the tier rules check the TCB, not the mathematics.
//!
//! The catalogue is that answer. Each entry names a theorem in a Lean source
//! file, records the SHA-256 of its statement, and says in a sentence what
//! the statement means. [`Catalogue::verify`] re-reads the Lean source and
//! recomputes the hash, so a theorem whose statement is weakened no longer
//! matches the entry that was approved, and CI says so.
//!
//! What this does *not* do is check that the English sentence describes the
//! Lean statement. A human reads those, once, when the entry is added. The
//! catalogue's job is to make sure the thing they read is the thing that is
//! still being proved.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use meden_core::digest::sha256_hex;
use serde::{Deserialize, Serialize};

/// Where a statement lives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// Path to the source file, relative to the Lean root.
    pub file: String,
    /// The theorem's name, as written.
    pub theorem: String,
}

/// One catalogued invariant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The property id an attestation may claim, e.g. `inv.v0.deny-wins`.
    pub id: String,
    /// What the statement means, for the human who approves the entry.
    pub means: String,
    /// Where the statement is.
    pub lean: Location,
    /// SHA-256 of the normalised statement text.
    pub statement_sha256: String,
}

/// The catalogue as a whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    /// Catalogue version, matching the `inv.<version>.` prefix of its ids.
    pub version: String,
    /// The entries.
    pub invariants: Vec<Entry>,
}

/// What checking one entry came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The source says what the catalogue says it says.
    Matches,
    /// The statement is there but is not the catalogued one.
    Changed {
        /// What the source hashes to now.
        found: String,
    },
    /// No theorem of that name in that file.
    Missing,
    /// The file could not be read.
    Unreadable(String),
}

impl Catalogue {
    /// Read a catalogue from JSON.
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading the catalogue at {}", path.display()))?;
        let catalogue: Catalogue = serde_json::from_str(&text)
            .with_context(|| format!("parsing the catalogue at {}", path.display()))?;
        catalogue.check_ids()?;
        Ok(catalogue)
    }

    /// Every id is unique and carries the catalogue's own version prefix.
    fn check_ids(&self) -> Result<()> {
        let prefix = format!("inv.{}.", self.version);
        let mut seen = BTreeSet::new();
        for entry in &self.invariants {
            if !entry.id.starts_with(&prefix) {
                bail!(
                    "{} is in a {} catalogue but does not start with {prefix:?}",
                    entry.id,
                    self.version
                );
            }
            if !seen.insert(entry.id.clone()) {
                bail!(
                    "{} appears twice; a property id must mean one thing",
                    entry.id
                );
            }
        }
        Ok(())
    }

    /// The entry with this id.
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.invariants.iter().find(|e| e.id == id)
    }

    /// Check every entry against the Lean sources under `root`.
    pub fn verify(&self, root: &Path) -> Vec<(&Entry, Status)> {
        self.invariants
            .iter()
            .map(|entry| (entry, entry.check(root)))
            .collect()
    }
}

impl Entry {
    /// The path this entry's statement should be read from.
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(&self.lean.file)
    }

    /// Compare the catalogued hash with what the source says now.
    pub fn check(&self, root: &Path) -> Status {
        let text = match fs::read_to_string(self.path(root)) {
            Ok(t) => t,
            Err(e) => return Status::Unreadable(e.to_string()),
        };
        match statement_of(&text, &self.lean.theorem) {
            None => Status::Missing,
            Some(statement) => {
                let found = sha256_hex(statement.as_bytes());
                if found == self.statement_sha256 {
                    Status::Matches
                } else {
                    Status::Changed { found }
                }
            }
        }
    }
}

/// The statement of a theorem, as a single normalised line.
///
/// Everything from `theorem <name>` up to the `:=` that starts the proof,
/// with runs of whitespace collapsed. Collapsing is deliberate: Lean's
/// formatter rewraps lines, and a hash that changed when a line was rewrapped
/// would be noise rather than a signal. Nothing else is normalised, so a
/// hypothesis that is weakened, dropped or renamed changes the hash.
pub fn statement_of(source: &str, theorem: &str) -> Option<String> {
    let mut rest = source;
    loop {
        let at = rest.find("theorem ")?;
        let after = rest.get(at.checked_add(8)?..)?;
        let name_len = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\'' || c == '.'))
            .unwrap_or(after.len());
        let name = after.get(..name_len)?;
        // A `theorem` that is part of a longer word is not this theorem, and
        // one inside a comment is not a statement; requiring the keyword to
        // start its line rules both out.
        let at_line_start = rest
            .get(..at)
            .is_some_and(|before| before.is_empty() || before.ends_with('\n'));
        if at_line_start && name == theorem {
            let body = after.get(name_len..)?;
            let end = find_assign(body)?;
            let whole = format!("theorem {name}{}", body.get(..end)?);
            return Some(normalise(&whole));
        }
        rest = rest.get(at.checked_add(8)?..)?;
    }
}

/// The byte offset of the `:=` that ends a statement.
///
/// Skips `:=` inside a bracketed group, which is where a default argument or
/// a structure literal would put one.
fn find_assign(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match *bytes.get(i)? {
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b':' if depth == 0 && bytes.get(i.checked_add(1)?) == Some(&b'=') => return Some(i),
            _ => {}
        }
        i = i.checked_add(1)?;
    }
    None
}

/// Collapse runs of whitespace to one space and trim.
fn normalise(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::*;

    const SOURCE: &str = r"
/-- A doc comment mentioning theorem foo in prose. -/
theorem foo (a : Nat) (h : a = 1) :
    a + 0 = 1 := by
  simp [h]

theorem bar : True := trivial
";

    #[test]
    fn a_statement_is_everything_up_to_the_proof() {
        let s = statement_of(SOURCE, "foo").expect("foo is there");
        assert_eq!(s, "theorem foo (a : Nat) (h : a = 1) : a + 0 = 1");
        assert!(!s.contains("simp"), "the proof leaked into the statement");
    }

    #[test]
    fn prose_that_mentions_a_theorem_is_not_a_theorem() {
        // The doc comment above `foo` says "theorem foo"; the extractor must
        // find the declaration, not the sentence.
        let s = statement_of(SOURCE, "foo").expect("foo is there");
        assert!(s.starts_with("theorem foo (a : Nat)"), "{s}");
    }

    #[test]
    fn a_name_that_is_not_there_is_not_invented() {
        assert_eq!(statement_of(SOURCE, "baz"), None);
    }

    #[test]
    fn rewrapping_a_line_does_not_change_the_hash() {
        let wrapped = "theorem foo (a : Nat)\n    (h : a = 1) :\n  a + 0 = 1 := by\n  simp [h]\n";
        assert_eq!(
            statement_of(SOURCE, "foo"),
            statement_of(wrapped, "foo"),
            "reformatting changed the statement"
        );
    }

    #[test]
    fn dropping_a_hypothesis_does_change_it() {
        let weaker = "theorem foo (a : Nat) : a + 0 = 1 := by simp\n";
        assert_ne!(
            statement_of(SOURCE, "foo"),
            statement_of(weaker, "foo"),
            "a weakened statement hashed the same"
        );
    }

    #[test]
    fn ids_must_be_unique_and_carry_the_version() {
        let dup = Catalogue {
            version: "v0".to_owned(),
            invariants: vec![
                Entry {
                    id: "inv.v0.a".to_owned(),
                    means: "m".to_owned(),
                    lean: Location {
                        file: "f".to_owned(),
                        theorem: "t".to_owned()
                    },
                    statement_sha256: "h".to_owned(),
                };
                2
            ],
        };
        assert!(dup.check_ids().is_err(), "a repeated id was accepted");

        let wrong_version = Catalogue {
            version: "v0".to_owned(),
            invariants: vec![Entry {
                id: "inv.v1.a".to_owned(),
                means: "m".to_owned(),
                lean: Location {
                    file: "f".to_owned(),
                    theorem: "t".to_owned(),
                },
                statement_sha256: "h".to_owned(),
            }],
        };
        assert!(
            wrong_version.check_ids().is_err(),
            "a v1 id in a v0 catalogue was accepted"
        );
    }
}
