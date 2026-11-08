//! The same trusted-key decision for every TUI approval surface.

use crossterm::event::{Event, KeyCode, KeyEventKind};

use super::Screen;

/// Approval target selected by the host's current review screen.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Choice {
    Processor,
    Export,
    Contract,
}

/// An approval key has authority only for the current host review state.
/// Handlers must match this target rather than interpreting `y` on their own.
pub(super) fn route(screen: &Screen, event: &Event) -> Option<Choice> {
    if !approves(event) {
        return None;
    }
    match screen {
        Screen::ReviewingProcessor => Some(Choice::Processor),
        Screen::ReviewingExport => Some(Choice::Export),
        Screen::ReviewingContract => Some(Choice::Contract),
        _ => None,
    }
}

fn approves(event: &Event) -> bool {
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

    #[test]
    fn approval_is_bound_to_the_current_review_target() {
        for (screen, target) in [
            (Screen::Picking, None),
            (Screen::Instructing, None),
            (Screen::Running, None),
            (Screen::ReviewingProcessor, Some(Choice::Processor)),
            (Screen::ReviewingExport, Some(Choice::Export)),
            (Screen::ReviewingContract, Some(Choice::Contract)),
        ] {
            for bits in 0..=u8::MAX {
                for kind in [
                    KeyEventKind::Press,
                    KeyEventKind::Repeat,
                    KeyEventKind::Release,
                ] {
                    for code in [KeyCode::Char('y'), KeyCode::Char('Y'), KeyCode::Enter] {
                        let event = Event::Key(KeyEvent::new_with_kind(
                            code,
                            KeyModifiers::from_bits_retain(bits),
                            kind,
                        ));
                        let actual = route(&screen, &event);
                        if bits == 0 && kind == KeyEventKind::Press && code == KeyCode::Char('y') {
                            assert_eq!(actual.as_ref(), target.as_ref());
                        } else {
                            assert_eq!(actual, None);
                        }
                    }
                }
            }
            for text in ["y", "approved=true", "\u{1b}[y"] {
                assert_eq!(route(&screen, &Event::Paste(text.into())), None);
            }
        }
    }
}
