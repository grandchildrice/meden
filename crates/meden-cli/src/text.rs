//! Render untrusted text without terminal or directional control effects.

pub fn visible(text: &str) -> String {
    text.chars()
        .map(|c| char::from_u32(visible_codepoint(c as u32)).unwrap_or('\u{fffd}'))
        .collect()
}

/// The display policy on numeric Unicode scalars. Numeric cases keep the
/// production transformation in the extractor's supported Rust subset.
fn visible_codepoint(code: u32) -> u32 {
    match code {
        0x0a => 0x0a,
        0x09 => 0x20,
        0x1b => 0x241b,
        0x061c
        | 0x200e
        | 0x200f
        | 0x202a..=0x202e
        | 0x2066..=0x2069
        | 0x00..=0x1f
        | 0x7f..=0x9f => 0xfffd,
        _ => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_unicode_scalar_keeps_the_original_display_policy() {
        for code in 0..=0x0010_ffff {
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            let expected = match c {
                '\n' => '\n',
                '\t' => ' ',
                '\u{1b}' => '␛',
                '\u{61c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}' => '�',
                c if c.is_control() => '�',
                c => c,
            };
            assert_eq!(visible_codepoint(code), expected as u32, "U+{code:04X}");
            assert_eq!(
                visible(&c.to_string()),
                expected.to_string(),
                "U+{code:04X}"
            );
        }
    }
}
