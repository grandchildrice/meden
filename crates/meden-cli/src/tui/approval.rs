//! The same trusted-key decision for every TUI approval surface.

use crossterm::event::{Event, KeyCode, KeyEventKind};

pub(super) fn approves(event: &Event) -> bool {
    if let Event::Key(key) = event {
        key.kind == KeyEventKind::Press
            && key.modifiers.is_empty()
            && key.code == KeyCode::Char('y')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    #[test]
    fn only_unmodified_key_presses_can_approve() {
        for bits in 0..=u8::MAX {
            for kind in [
                KeyEventKind::Press,
                KeyEventKind::Repeat,
                KeyEventKind::Release,
            ] {
                for code in [
                    KeyCode::Char('y'),
                    KeyCode::Char('Y'),
                    KeyCode::Char('ｙ'),
                    KeyCode::Enter,
                ] {
                    let event = Event::Key(KeyEvent::new_with_kind(
                        code,
                        KeyModifiers::from_bits_retain(bits),
                        kind,
                    ));
                    assert_eq!(
                        approves(&event),
                        bits == 0 && kind == KeyEventKind::Press && code == KeyCode::Char('y')
                    );
                }
            }
        }
        for text in ["y", "approved=true", "\u{1b}[y", "yes"] {
            assert!(!approves(&Event::Paste(text.into())));
        }
        assert!(!approves(&Event::FocusGained));
        assert!(!approves(&Event::Resize(121, 0)));
    }
}
