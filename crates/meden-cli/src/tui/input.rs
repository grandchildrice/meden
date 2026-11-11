//! Editable instruction buffer. Cursor offsets always fall between graphemes.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub(super) struct Input {
    text: String,
    cursor: usize,
    history: Vec<String>,
    recalled: Option<usize>,
    draft: String,
    killed: String,
    search: Option<Search>,
}

struct Search {
    query: String,
    original: String,
    cursor: usize,
    matched: Option<usize>,
}

pub(super) struct View {
    pub lines: Vec<String>,
    pub row: usize,
    pub column: usize,
}

impl Input {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn insert(&mut self, text: &str) {
        let text = super::visible(&text.replace("\r\n", "\n").replace('\r', "\n"));
        self.text.insert_str(self.cursor, &text);
        let end = self.cursor.saturating_add(text.len());
        // A combining character can join the inserted text to its neighbour.
        self.cursor = end;
        self.snap_cursor();
    }

    fn snap_cursor(&mut self) {
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|i| *i >= self.cursor)
            .unwrap_or(self.text.len());
    }

    pub fn left(&mut self) {
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .take_while(|i| *i < self.cursor)
            .last()
            .unwrap_or(0);
    }

    pub fn right(&mut self) {
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|i| *i > self.cursor)
            .unwrap_or(self.text.len());
    }

    pub fn backspace(&mut self) {
        let end = self.cursor;
        self.left();
        self.text.replace_range(self.cursor..end, "");
        self.snap_cursor();
    }

    pub fn delete(&mut self) {
        let start = self.cursor;
        self.right();
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.snap_cursor();
    }

    pub fn home(&mut self) {
        self.cursor = self
            .text
            .get(..self.cursor)
            .unwrap_or("")
            .rfind('\n')
            .map_or(0, |i| i.saturating_add(1));
    }

    pub fn end(&mut self) {
        self.cursor = self
            .text
            .get(self.cursor..)
            .unwrap_or("")
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor.saturating_add(i));
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.recalled = None;
        self.draft.clear();
        self.search = None;
    }

    fn kill(&mut self, start: usize, end: usize) {
        if start == end {
            return;
        }
        self.killed = self.text.get(start..end).unwrap_or_default().to_owned();
        self.text.replace_range(start..end, "");
        self.cursor = start;
        self.snap_cursor();
    }

    pub fn kill_before(&mut self) {
        let end = self.cursor;
        self.home();
        let start = if self.cursor == end {
            end.saturating_sub(1)
        } else {
            self.cursor
        };
        self.kill(start, end);
    }

    pub fn kill_after(&mut self) {
        let start = self.cursor;
        self.end();
        let end = if self.cursor == start {
            start.saturating_add(1).min(self.text.len())
        } else {
            self.cursor
        };
        self.kill(start, end);
    }

    pub fn word_left(&mut self) {
        self.cursor = self
            .text
            .get(..self.cursor)
            .unwrap_or_default()
            .split_word_bound_indices()
            .filter(|(_, word)| word.chars().any(char::is_alphanumeric))
            .map(|(index, _)| index)
            .next_back()
            .unwrap_or(0);
    }

    pub fn word_right(&mut self) {
        let rest = self.text.get(self.cursor..).unwrap_or_default();
        let offset = rest
            .split_word_bound_indices()
            .find(|(_, word)| word.chars().any(char::is_alphanumeric))
            .map_or(rest.len(), |(index, word)| index.saturating_add(word.len()));
        self.cursor = self.cursor.saturating_add(offset);
        self.snap_cursor();
    }

    pub fn kill_word(&mut self) {
        let end = self.cursor;
        let mut start = end;
        let mut in_word = false;
        for (index, grapheme) in self
            .text
            .get(..end)
            .unwrap_or_default()
            .grapheme_indices(true)
            .rev()
        {
            if grapheme.chars().all(char::is_whitespace) {
                if in_word {
                    break;
                }
            } else {
                in_word = true;
            }
            start = index;
        }
        self.kill(start, end);
    }

    pub fn yank(&mut self) {
        self.insert(&self.killed.clone());
    }

    pub fn search_query(&self) -> Option<&str> {
        self.search.as_ref().map(|s| s.query.as_str())
    }

    pub fn search_found(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.matched.is_some())
    }

    pub fn search_history(&mut self) {
        if self.search.is_none() {
            self.search = Some(Search {
                query: String::new(),
                original: self.text.clone(),
                cursor: self.cursor,
                matched: None,
            });
            self.find_history(false);
        } else {
            self.find_history(true);
        }
    }

    pub fn search_insert(&mut self, text: &str) {
        if let Some(search) = &mut self.search {
            search.query.push_str(&super::visible(text));
            self.find_history(false);
        }
    }

    pub fn search_backspace(&mut self) {
        if let Some(search) = &mut self.search {
            if let Some((index, _)) = search.query.grapheme_indices(true).next_back() {
                search.query.truncate(index);
            }
            self.find_history(false);
        }
    }

    fn find_history(&mut self, previous: bool) {
        let Some(search) = &mut self.search else {
            return;
        };
        let end = if previous {
            search.matched.unwrap_or(self.history.len())
        } else {
            self.history.len()
        };
        if let Some((index, text)) = self
            .history
            .iter()
            .enumerate()
            .take(end)
            .rev()
            .find(|(_, text)| text.contains(&search.query))
        {
            search.matched = Some(index);
            self.text.clone_from(text);
            self.cursor = self.text.len();
        } else {
            search.matched = None;
        }
    }

    pub fn finish_search(&mut self, cancel: bool) {
        if let Some(search) = self.search.take() {
            if cancel || search.matched.is_none() {
                self.text = search.original;
                self.cursor = search.cursor;
            }
            self.recalled = None;
        }
    }

    pub fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.text);
        if !text.trim().is_empty() && self.history.last() != Some(&text) {
            self.history.push(text.clone());
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.clear();
        text
    }

    // Up/down recall single-line instructions; multiline input moves by line.
    pub fn up(&mut self) {
        if self
            .text
            .get(..self.cursor)
            .unwrap_or_default()
            .contains('\n')
            && self.recalled.is_none()
        {
            self.vertical(false);
            return;
        }
        let index = match self.recalled {
            Some(i) => i.saturating_sub(1),
            None if !self.history.is_empty() => {
                self.draft.clone_from(&self.text);
                self.history.len().saturating_sub(1)
            }
            None => return,
        };
        self.recall(index);
    }

    pub fn down(&mut self) {
        if self
            .text
            .get(self.cursor..)
            .unwrap_or_default()
            .contains('\n')
            && self.recalled.is_none()
        {
            self.vertical(true);
            return;
        }
        if let Some(index) = self.recalled {
            if index.saturating_add(1) < self.history.len() {
                self.recall(index.saturating_add(1));
            } else {
                self.text.clone_from(&self.draft);
                self.cursor = self.text.len();
                self.recalled = None;
            }
        }
    }

    fn recall(&mut self, index: usize) {
        if let Some(text) = self.history.get(index) {
            self.text.clone_from(text);
            self.cursor = self.text.len();
            self.recalled = Some(index);
        }
    }

    fn vertical(&mut self, down: bool) {
        let before = self.text.get(..self.cursor).unwrap_or("");
        let start = before.rfind('\n').map_or(0, |i| i.saturating_add(1));
        let column = before.get(start..).unwrap_or("").width();
        let target = if down {
            let Some(end) = self.text.get(self.cursor..).unwrap_or("").find('\n') else {
                return;
            };
            self.cursor.saturating_add(end).saturating_add(1)
        } else if start > 0 {
            self.text
                .get(..start.saturating_sub(1))
                .unwrap_or("")
                .rfind('\n')
                .map_or(0, |i| i.saturating_add(1))
        } else {
            return;
        };
        let line = self
            .text
            .get(target..)
            .unwrap_or("")
            .split('\n')
            .next()
            .unwrap_or("");
        let mut width: usize = 0;
        self.cursor = target;
        for grapheme in line.graphemes(true) {
            width = width.saturating_add(grapheme.width());
            if width > column {
                break;
            }
            self.cursor = self.cursor.saturating_add(grapheme.len());
        }
    }

    /// Hard-wrap by display cells so the painted cursor and text agree even
    /// with Japanese, emoji, combining accents and a narrow terminal.
    pub fn view(&self, width: usize) -> View {
        let width = width.max(4);
        let mut lines = Vec::new();
        let mut line = "> ".to_owned();
        let mut column: usize = 2;
        let mut cursor = (0, 2);
        for (index, grapheme) in self.text.grapheme_indices(true) {
            let cells = grapheme.width();
            if column >= width || (grapheme != "\n" && column.saturating_add(cells) > width) {
                lines.push(std::mem::replace(&mut line, "  ".into()));
                column = 2;
            }
            if index == self.cursor {
                cursor = (lines.len(), column);
            }
            if grapheme == "\n" {
                lines.push(std::mem::replace(&mut line, "  ".into()));
                column = 2;
            } else {
                line.push_str(grapheme);
                column = column.saturating_add(cells);
            }
        }
        if self.cursor == self.text.len() {
            if column >= width {
                lines.push(std::mem::replace(&mut line, "  ".into()));
                column = 2;
            }
            cursor = (lines.len(), column);
        }
        lines.push(line);
        View {
            lines,
            row: cursor.0,
            column: cursor.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_and_yank_preserve_unicode_and_the_rest_of_the_line() {
        let mut input = Input::default();
        input.insert("hello 日本語 world");
        input.word_left();
        input.kill_before();
        assert_eq!(input.text(), "world");
        input.yank();
        assert_eq!(input.text(), "hello 日本語 world");
        input.kill_after();
        assert_eq!(input.text(), "hello 日本語 ");
        input.yank();
        input.end();
        input.kill_word();
        assert_eq!(input.text(), "hello 日本語 ");
        input.yank();
        assert_eq!(input.text(), "hello 日本語 world");
    }

    #[test]
    fn history_search_selects_without_submitting_and_cancel_restores_the_draft() {
        let mut input = Input::default();
        for text in ["first build", "second build", "test"] {
            input.insert(text);
            input.take();
        }
        input.insert("unsent 日本語");
        input.search_history();
        input.search_insert("build");
        assert_eq!(input.text(), "second build");
        input.search_history();
        assert_eq!(input.text(), "first build");
        input.finish_search(true);
        assert_eq!(input.text(), "unsent 日本語");
        input.search_history();
        input.search_insert("does not exist");
        assert!(!input.search_found());
        input.finish_search(false);
        assert_eq!(input.text(), "unsent 日本語");
        input.search_history();
        input.search_insert("second");
        input.finish_search(false);
        assert_eq!(input.text(), "second build");
        assert_eq!(input.history.len(), 3);
    }

    #[test]
    fn edit_graphemes_without_splitting_japanese_emoji_or_combining_marks() {
        let mut input = Input::default();
        input.insert("日e\u{301}👩‍💻本");
        input.left();
        input.backspace();
        assert_eq!(input.text(), "日e\u{301}本");
        input.left();
        input.delete();
        assert_eq!(input.text(), "日本");
        input.insert("語");
        assert_eq!(input.take(), "日語本");
        assert!(input.text().is_empty());
    }

    #[test]
    fn multiline_navigation_and_paste_keep_cursor_at_display_column() {
        let mut input = Input::default();
        input.insert("ab\r\n日本語\rxyz");
        input.up();
        input.insert("!");
        assert_eq!(input.text(), "ab\n日!本語\nxyz");
        input.home();
        input.delete();
        input.end();
        input.insert("?");
        input.down();
        input.insert(".");
        assert_eq!(input.text(), "ab\n!本語?\nxyz.");
    }

    #[test]
    fn history_recovers_unsent_draft_and_does_not_edit_previous_turn() {
        let mut input = Input::default();
        input.insert("first");
        input.take();
        input.insert("second");
        input.take();
        input.insert("draft");
        input.up();
        input.insert("!");
        input.up();
        assert_eq!(input.text(), "first");
        input.down();
        assert_eq!(input.text(), "second");
        input.down();
        assert_eq!(input.text(), "draft");
    }

    #[test]
    fn wrapping_counts_cells_and_reserves_a_cursor_at_the_end() {
        let mut input = Input::default();
        input.insert("日本ab");
        let view = input.view(6);
        assert_eq!(view.lines, ["> 日本", "  ab"]);
        assert_eq!((view.row, view.column), (1, 4));
        input.left();
        input.left();
        let view = input.view(6);
        assert_eq!((view.row, view.column), (1, 2));
        input.end();
        input.insert("cd");
        let view = input.view(6);
        assert_eq!(view.lines, ["> 日本", "  abcd", "  "]);
        assert_eq!((view.row, view.column), (2, 2));
    }

    #[test]
    fn cursor_stays_in_bounds_before_a_newline_after_a_full_row() {
        let mut input = Input::default();
        input.insert("日本\nabc");
        input.home();
        input.left();
        let view = input.view(6);
        assert!(view.column < 6);
        assert_eq!(view.lines.get(view.row), Some(&"  ".to_owned()));
    }
}
