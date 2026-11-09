//! Immutable Unicode scalars used by the policy kernel.

use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};

/// A scope or glob pattern, decoded when host data enters the policy model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Scope(Vec<char>);

impl Scope {
    /// Match this pattern against the exact decoded requirement scope.
    pub fn matches(&self, text: &Scope) -> bool {
        crate::glob::glob_match_chars(&self.0, &text.0)
    }
}

impl From<&str> for Scope {
    fn from(value: &str) -> Self {
        Self(value.chars().collect())
    }
}

impl From<String> for Scope {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<Vec<char>> for Scope {
    fn from(value: Vec<char>) -> Self {
        Self(value)
    }
}

impl From<Scope> for String {
    fn from(value: Scope) -> Self {
        value.0.into_iter().collect()
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in &self.0 {
            f.write_char(*ch)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_and_json_preserve_unicode_empty_and_literal_wildcards() -> serde_json::Result<()> {
        for length in [0, 1, 31, 32, 33, 65] {
            let text = "é🦀\0/*?".repeat(length);
            let scope = Scope::from(text.as_str());
            assert_eq!(scope.0, text.chars().collect::<Vec<_>>());
            assert_eq!(scope.to_string(), text);
            assert_eq!(String::from(scope.clone()), text);
            let json = serde_json::to_string(&scope)?;
            assert_eq!(serde_json::from_str::<String>(&json)?, text);
            assert_eq!(serde_json::from_str::<Scope>(&json)?, scope);
        }
        Ok(())
    }
}
