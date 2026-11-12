//! Presentation pacing for arriving text. Full replies, history and tool
//! admission never depend on the visible prefix or this animation's clock.

use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation as _;

pub(super) struct Reveal {
    shown: usize,
    updated: Instant,
    step: usize,
    complete: bool,
}

impl Reveal {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            shown: 1,
            updated: now,
            step: 1,
            complete: false,
        }
    }

    pub(super) fn finish(&mut self) {
        self.complete = true;
    }

    pub(super) fn prefix<'a>(&mut self, text: &'a str, now: Instant) -> &'a str {
        let total = text.graphemes(true).count();
        let ticks = now.saturating_duration_since(self.updated).as_millis() / 16;
        if ticks > 0 {
            let remaining = total.saturating_sub(self.shown);
            // Ordinary fragments arrive character by character. A large batch
            // catches up in about 320ms, without a long artificial typing wait.
            self.step = self.step.max(remaining.div_ceil(20)).max(1);
            self.shown = self.shown.saturating_add(
                self.step
                    .saturating_mul(usize::try_from(ticks).unwrap_or(usize::MAX)),
            );
            if self.shown >= total {
                self.step = 1;
            }
            self.updated = now;
        }
        self.shown = self.shown.min(total);
        let end = text
            .grapheme_indices(true)
            .nth(self.shown)
            .map_or(text.len(), |(index, _)| index);
        text.get(..end).unwrap_or_default()
    }

    pub(super) fn done(&self, text: &str) -> bool {
        self.complete && self.shown >= text.graphemes(true).count()
    }
}

pub(super) const FRAME: Duration = Duration::from_millis(16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_reveal_without_splitting_graphemes_and_catch_up_without_delaying_tools() {
        let now = Instant::now();
        let text = "日e\u{301}👨‍👩‍👧‍👦本";
        let mut reveal = Reveal::new(now);
        assert_eq!(reveal.prefix(text, now), "日");
        assert_eq!(reveal.prefix(text, now + FRAME), "日e\u{301}");
        reveal.finish();
        assert_eq!(reveal.prefix(text, now + Duration::from_secs(1)), text);
        assert!(reveal.done(text));
        // A replacement final message need not share provisional UTF-8 offsets.
        assert_eq!(reveal.prefix("✓", now + Duration::from_secs(1)), "✓");
        let batch = "あ".repeat(10_000);
        let mut reveal = Reveal::new(now);
        reveal.finish();
        for tick in 1..=20 {
            reveal.prefix(&batch, now + FRAME * tick);
        }
        assert!(reveal.done(&batch), "large replies must catch up in 320ms");
        let mut reveal = Reveal::new(now);
        reveal.prefix("a", now + Duration::from_secs(10));
        assert_eq!(
            reveal.prefix("abcdef", now + Duration::from_secs(10) + FRAME),
            "ab"
        );
    }
}
