use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use texera::EditorViewState;
use texera_core::{Selection, State, tr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchFocus {
    SearchInput,
    Editor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub from: usize, // character offset start
    pub to: usize,   // character offset end
    pub line: usize, // 0-based logical line number
}

#[derive(Clone, Debug)]
pub struct SearchState {
    pub is_active: bool,
    pub focus: SearchFocus,
    pub query: String,
    pub query_cursor: usize,
    pub matches: Vec<SearchMatch>,
    pub current_match_idx: Option<usize>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            is_active: false,
            focus: SearchFocus::SearchInput,
            query: String::new(),
            query_cursor: 0,
            matches: Vec::new(),
            current_match_idx: None,
        }
    }
}

impl SearchState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the search bar, optionally seeding the query with currently selected text.
    pub fn open(&mut self, state: &State) {
        self.is_active = true;
        self.focus = SearchFocus::SearchInput;

        let main_range = state.selection().main();
        if !main_range.empty() {
            let from = main_range.from();
            let to = main_range.to();
            let sel_text = state.doc().slice(from, Some(to)).to_string();
            // Avoid multiline queries
            if !sel_text.is_empty() && !sel_text.contains('\n') {
                self.query = sel_text;
                self.query_cursor = self.query.chars().count();
            }
        }

        self.recompute_matches(state);
    }

    /// Closes search mode and clears active matches.
    pub fn close(&mut self) {
        self.is_active = false;
        self.focus = SearchFocus::Editor;
        self.matches.clear();
        self.current_match_idx = None;
    }

    /// Splits the current search query at the cursor position.
    pub fn query_split_at_cursor(&self) -> (String, String) {
        let chars: Vec<char> = self.query.chars().collect();
        let cursor = self.query_cursor.min(chars.len());
        let before: String = chars[..cursor].iter().collect();
        let after: String = chars[cursor..].iter().collect();
        (before, after)
    }

    /// Recomputes matches across the document for the current query.
    pub fn recompute_matches(&mut self, state: &State) {
        self.matches.clear();
        self.current_match_idx = None;

        if self.query.is_empty() {
            return;
        }

        let doc_str = state.doc().to_string();
        let query_lower = self.query.to_lowercase();
        let doc_lower = doc_str.to_lowercase();

        let mut byte_offset = 0;
        let mut char_offset = 0;
        let mut current_line = 0;

        let query_char_len = self.query.chars().count();

        while let Some(found_byte) = doc_lower[byte_offset..].find(&query_lower) {
            let abs_byte = byte_offset + found_byte;

            // Advance char_offset and line count up to abs_byte
            let slice = &doc_str[byte_offset..abs_byte];
            for c in slice.chars() {
                char_offset += 1;
                if c == '\n' {
                    current_line += 1;
                }
            }

            let match_from = char_offset;
            let match_to = match_from + query_char_len;

            self.matches.push(SearchMatch {
                from: match_from,
                to: match_to,
                line: current_line,
            });

            // Advance through query
            let query_match_slice = &doc_str[abs_byte..abs_byte + query_lower.len()];
            for c in query_match_slice.chars() {
                char_offset += 1;
                if c == '\n' {
                    current_line += 1;
                }
            }

            byte_offset = abs_byte + query_lower.len();
        }

        if !self.matches.is_empty() {
            // Find match closest to current selection head
            let head = state.selection().main().head();
            let mut closest_idx = 0;
            let mut min_dist = usize::MAX;

            for (idx, m) in self.matches.iter().enumerate() {
                let dist = if m.from >= head {
                    m.from - head
                } else {
                    (head - m.from) + 10_000 // Prefer matches ahead of cursor
                };
                if dist < min_dist {
                    min_dist = dist;
                    closest_idx = idx;
                }
            }

            self.current_match_idx = Some(closest_idx);
        }
    }

    /// Advances to the next match and selects it in the editor.
    pub fn next_match(&mut self, state: &mut State) -> bool {
        if self.matches.is_empty() {
            return false;
        }

        let next_idx = match self.current_match_idx {
            Some(curr) => (curr + 1) % self.matches.len(),
            None => 0,
        };

        self.current_match_idx = Some(next_idx);
        self.apply_current_match(state);
        true
    }

    /// Retreats to the previous match and selects it in the editor.
    pub fn prev_match(&mut self, state: &mut State) -> bool {
        if self.matches.is_empty() {
            return false;
        }

        let prev_idx = match self.current_match_idx {
            Some(curr) => {
                if curr == 0 {
                    self.matches.len() - 1
                } else {
                    curr - 1
                }
            }
            None => self.matches.len() - 1,
        };

        self.current_match_idx = Some(prev_idx);
        self.apply_current_match(state);
        true
    }

    /// Updates the editor selection to highlight the current match.
    pub fn apply_current_match(&self, state: &mut State) {
        if let Some(idx) = self.current_match_idx {
            if let Some(m) = self.matches.get(idx) {
                let sel = Selection::range(m.from, m.to);
                if let Ok(res) = state.clone().update(tr!({
                    selection: sel,
                    scroll_into_view: true
                })) {
                    *state = res.into_state();
                }
            }
        }
    }

    /// Renders subtle highlights onto the editor buffer for visible matches.
    pub fn render_subtle_highlights(
        &self,
        buf: &mut Buffer,
        editor_area: Rect,
        state: &State,
        view_state: &EditorViewState,
        gutter_width: u16,
    ) {
        if !self.is_active || self.matches.is_empty() {
            return;
        }

        let scroll_y = view_state.scroll_y();
        let scroll_x = view_state.scroll_x();
        let text_start_x = editor_area.x + gutter_width;
        let text_area_width = editor_area.width.saturating_sub(gutter_width);

        if text_area_width == 0 {
            return;
        }

        let subtle_style = Style::default()
            .bg(Color::Rgb(45, 55, 75)) // Subtle slate blue background
            .fg(Color::Rgb(220, 230, 245));

        let current_style = Style::default()
            .bg(Color::Rgb(85, 75, 25)) // Subtle warm amber background for active match
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let doc = state.doc();

        for (idx, m) in self.matches.iter().enumerate() {
            let is_current = Some(idx) == self.current_match_idx;
            let style = if is_current { current_style } else { subtle_style };

            if let Ok(line) = doc.line(m.line) {
                if m.line < scroll_y {
                    continue;
                }
                let visual_row = m.line - scroll_y;
                if visual_row >= editor_area.height as usize {
                    continue;
                }

                let line_from = line.from();
                let match_start_col = m.from.saturating_sub(line_from);
                let match_end_col = m.to.saturating_sub(line_from);

                let y = editor_area.y + visual_row as u16;

                for col in match_start_col..match_end_col {
                    if col < scroll_x {
                        continue;
                    }
                    let visual_col = col - scroll_x;
                    if visual_col >= text_area_width as usize {
                        break;
                    }

                    let x = text_start_x + visual_col as u16;
                    if x < editor_area.right() && y < editor_area.bottom() {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_style(style);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_matches() {
        let state = State::from("foo bar baz\nhello foo world\n");
        let mut search = SearchState::new();
        search.query = "foo".to_string();
        search.recompute_matches(&state);

        assert_eq!(search.matches.len(), 2);
        assert_eq!(search.matches[0].from, 0);
        assert_eq!(search.matches[0].to, 3);
        assert_eq!(search.matches[0].line, 0);

        assert_eq!(search.matches[1].from, 18);
        assert_eq!(search.matches[1].to, 21);
        assert_eq!(search.matches[1].line, 1);
    }

    #[test]
    fn test_query_split_at_cursor() {
        let mut search = SearchState::new();
        search.query = "config".to_string();
        search.query_cursor = 3;
        let (before, after) = search.query_split_at_cursor();
        assert_eq!(before, "con");
        assert_eq!(after, "fig");

        search.query_cursor = 6;
        let (before2, after2) = search.query_split_at_cursor();
        assert_eq!(before2, "config");
        assert_eq!(after2, "");
    }

    #[test]
    fn test_search_navigation() {
        let mut state = State::from("one two one three one\n");
        let mut search = SearchState::new();
        search.query = "one".to_string();
        search.recompute_matches(&state);
        assert_eq!(search.matches.len(), 3);
        assert_eq!(search.current_match_idx, Some(0));

        search.next_match(&mut state);
        assert_eq!(search.current_match_idx, Some(1));

        search.prev_match(&mut state);
        assert_eq!(search.current_match_idx, Some(0));

        search.prev_match(&mut state);
        assert_eq!(search.current_match_idx, Some(2));
    }

    #[test]
    fn test_search_open_with_selected_text() {
        let mut state = State::from("configuration option enabled\n");
        let sel = Selection::range(14, 20);
        state = state.clone().update(tr!({ selection: sel })).unwrap().into_state();

        let mut search = SearchState::new();
        search.open(&state);
        assert_eq!(search.query, "option");
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].from, 14);
    }
}
