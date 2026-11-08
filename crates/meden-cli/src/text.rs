//! Render untrusted text without terminal or directional control effects.

pub fn visible(text: &str) -> String {
    text.chars()
        .map(|c| match c {
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
        })
        .collect()
}
