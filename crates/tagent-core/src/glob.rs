//! Scope matching.
//!
//! A grant's scope is a pattern over Unicode scalar values where `*` matches
//! any sequence (including the empty one and `/`), `?` matches exactly one
//! character and every other character matches itself. There are no
//! character classes and no escapes; the vocabulary is small on purpose so
//! that the Lean model (`TAgentSpec.Glob`) can define the same relation by
//! structural recursion and the two can be compared exhaustively on small
//! alphabets.
//!
//! The implementation is the classic iterative two-pointer algorithm with a
//! single backtrack point, so it uses constant extra space and cannot
//! overflow the stack on adversarial patterns.

/// Does `pattern` match `text`?
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_match_chars(&p, &t)
}

/// [`glob_match`] on already decoded characters.
pub fn glob_match_chars(p: &[char], t: &[char]) -> bool {
    let mut pi = 0usize; // index into pattern
    let mut ti = 0usize; // index into text
    let mut star: Option<(usize, usize)> = None; // (pattern idx after '*', text idx)

    while ti < t.len() {
        match (p.get(pi), t.get(ti)) {
            (Some('*'), _) => {
                // Record the backtrack point and try to match the rest of
                // the pattern against the rest of the text.
                pi = pi.saturating_add(1);
                star = Some((pi, ti));
            }
            (Some(pc), Some(tc)) if *pc == '?' || pc == tc => {
                pi = pi.saturating_add(1);
                ti = ti.saturating_add(1);
            }
            _ => match star {
                // Let the last '*' absorb one more character.
                Some((spi, sti)) => {
                    let next = sti.saturating_add(1);
                    pi = spi;
                    ti = next;
                    star = Some((spi, next));
                }
                None => return false,
            },
        }
    }
    // Only trailing stars may remain.
    while p.get(pi) == Some(&'*') {
        pi = pi.saturating_add(1);
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    proptest! {
        #[test]
        fn agrees_with_reference(p in "[ab*?]{0,6}", t in "[ab]{0,6}") {
            let pc: Vec<char> = p.chars().collect();
            let tc: Vec<char> = t.chars().collect();
            prop_assert_eq!(glob_match_chars(&pc, &tc), reference(&pc, &tc));
        }

        #[test]
        fn literal_pattern_matches_only_itself(p in "[a-z/.]{0,8}", t in "[a-z/.]{0,8}") {
            prop_assert_eq!(glob_match(&p, &t), p == t);
        }
    }
}
