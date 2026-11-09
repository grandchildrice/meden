//! Scope matching.
//!
//! A grant's scope is a pattern over Unicode scalar values where `*` matches
//! any sequence (including the empty one and `/`), `?` matches exactly one
//! character and every other character matches itself. There are no
//! character classes and no escapes; the vocabulary is small on purpose so
//! that the Lean model (`MedenSpec.Glob`) can define the same relation by
//! structural recursion and the two can be compared exhaustively on small
//! alphabets.
//!
//! Both public entry points use one dynamic-programming matcher. A row stores
//! the remaining pattern's result for each text suffix. Rows are updated from
//! right to left, using O(text length) space and no recursive Rust calls.

// Prepend one pattern character to the already evaluated suffix pattern.
// The caller supplies exactly one cell per text character.
// GlobRefinement.advance_row_refines proves the subtraction/access bounds
// under that invariant, which glob_match_chars establishes and preserves.
#[allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]
fn advance_row(pc: char, text: &[char], mut row: Vec<bool>, empty: bool) -> (Vec<bool>, bool) {
    let new_empty = pc == '*' && empty;
    let mut old_right = empty;
    let mut new_right = new_empty;
    let mut index = text.len();
    while index > 0 {
        index -= 1;
        let old = row[index];
        let value = if pc == '*' {
            old || new_right
        } else {
            (pc == '?' || pc == text[index]) && old_right
        };
        row[index] = value;
        old_right = old;
        new_right = value;
    }
    (row, new_empty)
}

/// Match decoded Unicode scalars for [`crate::scope::Scope::matches`].
/// Supports arbitrary `*`/`?` patterns, including empty inputs and `/`.
/// `GlobRefinement.glob_match_chars_refines` proves all subtraction/access bounds.
#[allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]
pub fn glob_match_chars(pattern: &[char], text: &[char]) -> bool {
    let mut row = vec![false; text.len()];
    let mut empty = true;
    let mut index = pattern.len();
    while index > 0 {
        index -= 1;
        (row, empty) = advance_row(pattern[index], text, row, empty);
    }
    if text.is_empty() {
        empty
    } else {
        row[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Exercise the production scope-decoding boundary before the scalar kernel.
    fn glob_match(pattern: &str, text: &str) -> bool {
        let pattern = crate::scope::Scope::from(pattern);
        let text = crate::scope::Scope::from(text);
        pattern.matches(&text)
    }

    /// The reference definition by structural recursion, mirroring the Lean
    /// model. Used only in tests.
    fn reference(p: &[char], t: &[char]) -> bool {
        match (p.split_first(), t.split_first()) {
            (None, None) => true,
            (Some(('*', pr)), _) => {
                reference(pr, t) || t.split_first().is_some_and(|(_, tr)| reference(p, tr))
            }
            (None, Some(_)) | (Some(_), None) => false,
            (Some(('?', pr)), Some((_, tr))) => reference(pr, tr),
            (Some((pc, pr)), Some((tc, tr))) => pc == tc && reference(pr, tr),
        }
    }

    #[test]
    fn basics() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "a"));
        assert!(glob_match("*", ""));
        assert!(glob_match("*", "anything/at/all"));
        assert!(glob_match("/workspace/*", "/workspace/a/b.txt"));
        assert!(!glob_match("/workspace/*", "/home/x"));
        assert!(glob_match("*.example.com", "mail.example.com"));
        assert!(!glob_match("*.example.com", "example.com"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(glob_match("a*b*c", "aXXbYYc"));
        assert!(!glob_match("a*b*c", "aXXbYY"));
        assert!(glob_match("**", "x"));
        assert!(glob_match("日本*", "日本語"));
    }

    #[test]
    fn all_small_unicode_scopes_match_the_recursive_language() {
        fn words(alphabet: &[char]) -> Vec<Vec<char>> {
            let mut result = vec![Vec::new()];
            let mut level = vec![Vec::new()];
            for _ in 0..3 {
                let mut next = Vec::new();
                for prefix in &level {
                    for ch in alphabet {
                        let mut word = prefix.clone();
                        word.push(*ch);
                        next.push(word);
                    }
                }
                result.extend(next.iter().cloned());
                level = next;
            }
            result
        }
        let patterns = words(&['a', '*', '?', 'é']);
        let texts = words(&['a', 'é', '🦀', '*', '?', '\0']);
        for pattern in &patterns {
            for text in &texts {
                let expected = reference(pattern, text);
                assert_eq!(
                    glob_match_chars(pattern, text),
                    expected,
                    "{pattern:?} / {text:?}"
                );
                assert_eq!(
                    glob_match(
                        &pattern.iter().collect::<String>(),
                        &text.iter().collect::<String>()
                    ),
                    expected,
                    "{pattern:?} / {text:?}"
                );
            }
        }
    }

    proptest! {
        #[test]
        fn agrees_with_reference(p in "[ab*?]{0,6}", t in "[ab]{0,6}") {
            let pc: Vec<char> = p.chars().collect();
            let tc: Vec<char> = t.chars().collect();
            prop_assert_eq!(glob_match_chars(&pc, &tc), reference(&pc, &tc));
        }

        #[test]
        fn str_version_agrees_with_reference(p in "[ab*?]{0,6}", t in "[ab]{0,6}") {
            let pc: Vec<char> = p.chars().collect();
            let tc: Vec<char> = t.chars().collect();
            prop_assert_eq!(glob_match(&p, &t), reference(&pc, &tc));
        }

        #[test]
        fn literal_pattern_matches_only_itself(p in "[a-z/.]{0,8}", t in "[a-z/.]{0,8}") {
            prop_assert_eq!(glob_match(&p, &t), p == t);
        }
    }
}
