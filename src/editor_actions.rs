use std::collections::BTreeSet;
use mynox::Error;
use texera_core::{ChangeSpec, Selection, SelectionRange, State, Text, TransactionSpec};

/// Determines the single-line comment token based on detected language name.
pub fn get_comment_prefix(lang: Option<&str>) -> &'static str {
    match lang.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("rust") | Some("c") | Some("cpp") | Some("javascript") | Some("typescript")
        | Some("java") | Some("go") | Some("csharp") | Some("php") | Some("swift")
        | Some("kotlin") => "//",
        Some("lua") | Some("sql") => "--",
        _ => "#", // Standard default for Linux/Unix config files (toml, yaml, ini, sh, conf, etc.)
    }
}

/// Clones the lines where cursors are positioned downwards or upwards.
pub fn clone_lines(state: &mut State, down: bool) -> Result<bool, Error> {
    let doc = state.doc();
    let num_lines = doc.len_lines();
    if num_lines == 0 {
        return Ok(false);
    }

    // Collect all line numbers covered by selections
    let mut line_indices = BTreeSet::new();
    for range in state.selection().ranges_ref() {
        if let Ok(line) = doc.line_at(range.from()) {
            let start_l = line.number();
            let end_l = doc.line_at(range.to()).map(|l| l.number()).unwrap_or(start_l);
            for l in start_l..=end_l {
                line_indices.insert(l);
            }
        }
    }

    if line_indices.is_empty() {
        return Ok(false);
    }

    // Build the cloned text block
    let mut cloned_text = String::new();
    let first_line = *line_indices.iter().next().unwrap();
    let last_line = *line_indices.iter().next_back().unwrap();

    for &line_idx in &line_indices {
        let line = doc.line(line_idx).map_err(|e| {
            mynox::error!("editor::clone", "Line access error", format!("{}", e))
        })?;
        cloned_text.push_str(&line.text().to_string());
        if !cloned_text.ends_with('\n') {
            cloned_text.push('\n');
        }
    }

    let insert_pos;
    let new_selection_shift;

    if down {
        let last = doc.line(last_line).map_err(|e| {
            mynox::error!("editor::clone", "Line access error", format!("{}", e))
        })?;
        insert_pos = last.to();
        let mut text_to_insert = String::new();
        if !last.text().ends_with('\n') {
            text_to_insert.push('\n');
            text_to_insert.push_str(cloned_text.trim_end_matches('\n'));
        } else {
            text_to_insert = cloned_text;
        }

        let change = ChangeSpec::new(insert_pos, Some(insert_pos), Some(Text::from(text_to_insert.as_str())))
            .map_err(|e| mynox::error!("editor::clone", "Transaction error", format!("{}", e)))?;

        // Calculate offset difference to move cursors to cloned block
        let target_line = doc.line(first_line).unwrap();
        new_selection_shift = (insert_pos + 1).saturating_sub(target_line.from());

        let mut new_ranges = Vec::new();
        for range in state.selection().ranges_ref() {
            let from = range.from() + new_selection_shift;
            let to = range.to() + new_selection_shift;
            new_ranges.push(SelectionRange::range(from, to));
        }

        let spec = TransactionSpec::default()
            .with_change(change)
            .with_selection(Selection::from(new_ranges));

        let res = state.clone().update(spec).map_err(|e| {
            mynox::error!("editor::clone", "Update error", format!("{}", e))
        })?;
        *state = res.into_state();
        Ok(true)
    } else {
        let first = doc.line(first_line).map_err(|e| {
            mynox::error!("editor::clone", "Line access error", format!("{}", e))
        })?;
        insert_pos = first.from();

        let change = ChangeSpec::new(insert_pos, Some(insert_pos), Some(Text::from(cloned_text.as_str())))
            .map_err(|e| mynox::error!("editor::clone", "Transaction error", format!("{}", e)))?;

        let spec = TransactionSpec::default().with_change(change);
        let res = state.clone().update(spec).map_err(|e| {
            mynox::error!("editor::clone", "Update error", format!("{}", e))
        })?;
        *state = res.into_state();
        Ok(true)
    }
}

/// Moves the selected logical lines one line down or one line up.
pub fn move_selection_lines(state: &mut State, down: bool) -> Result<bool, Error> {
    let doc = state.doc();
    let num_lines = doc.len_lines();
    if num_lines <= 1 {
        return Ok(false);
    }

    let mut line_indices = BTreeSet::new();
    for range in state.selection().ranges_ref() {
        if let Ok(line) = doc.line_at(range.from()) {
            let start_l = line.number();
            let end_l = doc.line_at(range.to()).map(|l| l.number()).unwrap_or(start_l);
            for l in start_l..=end_l {
                line_indices.insert(l);
            }
        }
    }

    if line_indices.is_empty() {
        return Ok(false);
    }

    let first_line = *line_indices.iter().next().unwrap();
    let last_line = *line_indices.iter().next_back().unwrap();

    if down {
        if last_line + 1 >= num_lines {
            return Ok(false); // Already at bottom
        }

        let target_line_idx = last_line + 1;
        let moving_first = doc.line(first_line).unwrap();
        let target_line = doc.line(target_line_idx).unwrap();

        let mut moving_text = String::new();
        for &l in &line_indices {
            moving_text.push_str(&doc.line(l).unwrap().text().to_string());
            if !moving_text.ends_with('\n') {
                moving_text.push('\n');
            }
        }

        let mut target_text = target_line.text().to_string();
        let had_trailing_newline = target_text.ends_with('\n');
        if !had_trailing_newline {
            target_text.push('\n');
        }

        let mut combined = target_text;
        combined.push_str(&moving_text);
        if !had_trailing_newline {
            combined = combined.trim_end_matches('\n').to_string();
        }

        let replace_from = moving_first.from();
        let replace_to = target_line.to();

        let change = ChangeSpec::new(replace_from, Some(replace_to), Some(Text::from(combined.as_str())))
            .map_err(|e| mynox::error!("editor::move", "Transaction error", format!("{}", e)))?;

        let target_line_len = doc.line(target_line_idx).unwrap().length();
        let mut new_ranges = Vec::new();
        for range in state.selection().ranges_ref() {
            new_ranges.push(SelectionRange::range(
                range.from() + target_line_len,
                range.to() + target_line_len,
            ));
        }

        let spec = TransactionSpec::default()
            .with_change(change)
            .with_selection(Selection::from(new_ranges));

        let res = state.clone().update(spec).map_err(|e| {
            mynox::error!("editor::move", "Update error", format!("{}", e))
        })?;
        *state = res.into_state();
        Ok(true)
    } else {
        if first_line == 0 {
            return Ok(false); // Already at top
        }

        let target_line_idx = first_line - 1;
        let target_line = doc.line(target_line_idx).unwrap();
        let moving_last = doc.line(last_line).unwrap();

        let mut target_text = target_line.text().to_string();
        if !target_text.ends_with('\n') {
            target_text.push('\n');
        }

        let mut moving_text = String::new();
        for &l in &line_indices {
            moving_text.push_str(&doc.line(l).unwrap().text().to_string());
            if !moving_text.ends_with('\n') {
                moving_text.push('\n');
            }
        }

        let mut combined = moving_text;
        combined.push_str(&target_text);
        if !moving_last.text().ends_with('\n') && target_line_idx + line_indices.len() >= num_lines - 1 {
            combined = combined.trim_end_matches('\n').to_string();
        }

        let replace_from = target_line.from();
        let replace_to = moving_last.to();

        let change = ChangeSpec::new(replace_from, Some(replace_to), Some(Text::from(combined.as_str())))
            .map_err(|e| mynox::error!("editor::move", "Transaction error", format!("{}", e)))?;

        let target_line_len = target_line.length();
        let mut new_ranges = Vec::new();
        for range in state.selection().ranges_ref() {
            new_ranges.push(SelectionRange::range(
                range.from().saturating_sub(target_line_len),
                range.to().saturating_sub(target_line_len),
            ));
        }

        let spec = TransactionSpec::default()
            .with_change(change)
            .with_selection(Selection::from(new_ranges));

        let res = state.clone().update(spec).map_err(|e| {
            mynox::error!("editor::move", "Update error", format!("{}", e))
        })?;
        *state = res.into_state();
        Ok(true)
    }
}

/// Deletes the lines where cursors/selections are located.
pub fn delete_lines(state: &mut State) -> Result<bool, Error> {
    let doc = state.doc();
    let num_lines = doc.len_lines();
    if num_lines == 0 {
        return Ok(false);
    }

    let mut line_indices = BTreeSet::new();
    for range in state.selection().ranges_ref() {
        if let Ok(line) = doc.line_at(range.from()) {
            let start_l = line.number();
            let end_l = doc.line_at(range.to()).map(|l| l.number()).unwrap_or(start_l);
            for l in start_l..=end_l {
                line_indices.insert(l);
            }
        }
    }

    if line_indices.is_empty() {
        return Ok(false);
    }

    let first_line = *line_indices.iter().next().unwrap();
    let last_line = *line_indices.iter().next_back().unwrap();

    let first = doc.line(first_line).unwrap();
    let last = doc.line(last_line).unwrap();

    let from;
    let to;

    if last_line + 1 < num_lines {
        // Delete from line start to next line start
        from = first.from();
        to = doc.line(last_line + 1).unwrap().from();
    } else if first_line > 0 {
        // Last line: delete from end of previous line to end of this line
        from = doc.line(first_line - 1).unwrap().to();
        to = last.to();
    } else {
        // Document has only these lines
        from = 0;
        to = doc.len_chars();
    }

    let change = ChangeSpec::new(from, Some(to), Some(Text::from("")))
        .map_err(|e| mynox::error!("editor::delete", "Transaction error", format!("{}", e)))?;

    let spec = TransactionSpec::default()
        .with_change(change)
        .with_selection(Selection::cursor(from.min(doc.len_chars().saturating_sub(to - from))));

    let res = state.clone().update(spec).map_err(|e| {
        mynox::error!("editor::delete", "Update error", format!("{}", e))
    })?;
    *state = res.into_state();
    Ok(true)
}

/// Adds a line below or above without breaking the current line, positioning the cursor in it.
pub fn add_line(state: &mut State, below: bool) -> Result<bool, Error> {
    let doc = state.doc();
    let mut new_ranges = Vec::new();
    let mut changes = Vec::new();

    // Group cursor heads by line to avoid duplicates on the same line
    let mut processed_lines = BTreeSet::new();

    for range in state.selection().ranges_ref() {
        if let Ok(line) = doc.line_at(range.head()) {
            if !processed_lines.insert(line.number()) {
                continue;
            }

            if below {
                let pos = line.to();
                let text = "\n";

                let cursor_pos = if line.text().ends_with('\n') {
                    pos
                } else {
                    pos + 1
                };

                // Position at new line without breaking existing line
                new_ranges.push(SelectionRange::cursor(cursor_pos));
                changes.push((pos, text));
            } else {
                let pos = line.from();
                new_ranges.push(SelectionRange::cursor(pos));
                changes.push((pos, "\n"));
            }
        }
    }

    if changes.is_empty() {
        return Ok(false);
    }

    // Sort changes descending by position so edits don't invalidate lower offsets
    changes.sort_by(|a, b| b.0.cmp(&a.0));

    let mut spec = TransactionSpec::default();
    for (pos, insert) in changes {
        let ch = ChangeSpec::new(pos, Some(pos), Some(Text::from(insert)))
            .map_err(|e| mynox::error!("editor::add_line", "Transaction error", format!("{}", e)))?;
        spec = spec.with_change(ch);
    }

    if !new_ranges.is_empty() {
        spec = spec.with_selection(Selection::from(new_ranges));
    }

    let res = state.clone().update(spec).map_err(|e| {
        mynox::error!("editor::add_line", "Update error", format!("{}", e))
    })?;
    *state = res.into_state();
    Ok(true)
}

/// Toggles comments on the lines spanned by the active selections.
/// Inserts `# ` or `// ` right before the first non-whitespace character.
pub fn toggle_comments(state: &mut State, lang: Option<&str>) -> Result<bool, Error> {
    let prefix = get_comment_prefix(lang);
    let doc = state.doc();

    let mut line_indices = BTreeSet::new();
    for range in state.selection().ranges_ref() {
        if let Ok(line) = doc.line_at(range.from()) {
            let start_l = line.number();
            let end_l = doc.line_at(range.to()).map(|l| l.number()).unwrap_or(start_l);
            for l in start_l..=end_l {
                line_indices.insert(l);
            }
        }
    }

    if line_indices.is_empty() {
        return Ok(false);
    }

    // Check if ALL non-empty lines are already commented
    let mut all_commented = true;
    let mut has_non_empty = false;

    for &idx in &line_indices {
        let line = doc.line(idx).unwrap();
        let line_text = line.text().to_string();
        let trimmed = line_text.trim_start();
        if !trimmed.is_empty() && trimmed != "\n" && trimmed != "\r\n" {
            has_non_empty = true;
            if !trimmed.starts_with(prefix) {
                all_commented = false;
                break;
            }
        }
    }

    let should_uncomment = has_non_empty && all_commented;

    // Apply line by line in reverse order to preserve offsets
    let mut changes = Vec::new();
    for &idx in line_indices.iter().rev() {
        let line = doc.line(idx).unwrap();
        let line_text = line.text().to_string();
        let line_start = line.from();

        // Calculate leading indentation
        let indent_len = line_text.chars().take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r').count();

        if should_uncomment {
            // Remove prefix and optional single space immediately following it
            let after_indent = &line_text[indent_len..];
            if after_indent.starts_with(prefix) {
                let comment_char_len = prefix.chars().count();
                let has_space = after_indent[prefix.len()..].starts_with(' ');
                let remove_len = if has_space { comment_char_len + 1 } else { comment_char_len };

                let from = line_start + indent_len;
                let to = from + remove_len;
                changes.push(ChangeSpec::new(from, Some(to), Some(Text::from(""))).unwrap());
            }
        } else {
            // Comment: insert prefix + single space right at first non-whitespace
            let from = line_start + indent_len;
            let insert = format!("{} ", prefix);
            changes.push(ChangeSpec::new(from, Some(from), Some(Text::from(insert.as_str()))).unwrap());
        }
    }

    let mut spec = TransactionSpec::default();
    for ch in changes {
        spec = spec.with_change(ch);
    }

    let res = state.clone().update(spec).map_err(|e| {
        mynox::error!("editor::toggle_comments", "Update error", format!("{}", e))
    })?;
    *state = res.into_state();
    Ok(true)
}

/// Inserts a literal hard tab (`\t`) replacing current selection(s).
pub fn insert_hard_tab(state: &mut State) -> Result<bool, Error> {
    let spec = state.replace_selection("\t");
    let res = state.clone().update(spec).map_err(|e| {
        mynox::error!("editor::hard_tab", "Update error", format!("{}", e))
    })?;

    *state = res.into_state();
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_comments_hash() {
        let mut state = State::from("    port = 80\n    host = localhost\n");
        let res = toggle_comments(&mut state, Some("toml"));
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "    # port = 80\n    host = localhost\n");

        // Toggle back (uncomment)
        let res2 = toggle_comments(&mut state, Some("toml"));
        assert!(res2.is_ok());
        assert_eq!(state.doc().to_string(), "    port = 80\n    host = localhost\n");
    }

    #[test]
    fn test_toggle_comments_slash() {
        let mut state = State::from("let x = 10;");
        let res = toggle_comments(&mut state, Some("rust"));
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "// let x = 10;");

        let res2 = toggle_comments(&mut state, Some("rust"));
        assert!(res2.is_ok());
        assert_eq!(state.doc().to_string(), "let x = 10;");
    }

    #[test]
    fn test_clone_lines_down() {
        let mut state = State::from("first\nsecond\n");
        let res = clone_lines(&mut state, true);
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "first\nfirst\nsecond\n");
    }

    #[test]
    fn test_clone_lines_up() {
        let mut state = State::from("first\nsecond\n");
        let res = clone_lines(&mut state, false);
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "first\nfirst\nsecond\n");
    }

    #[test]
    fn test_move_selection_down_and_up() {
        let mut state = State::from("line 1\nline 2\nline 3\n");
        // Move line 1 down -> line 2, line 1, line 3
        let res = move_selection_lines(&mut state, true);
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "line 2\nline 1\nline 3\n");

        // Move back up
        let res2 = move_selection_lines(&mut state, false);
        assert!(res2.is_ok());
        assert_eq!(state.doc().to_string(), "line 1\nline 2\nline 3\n");
    }

    #[test]
    fn test_delete_lines() {
        let mut state = State::from("line 1\nline 2\nline 3\n");
        let res = delete_lines(&mut state);
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "line 2\nline 3\n");
    }

    #[test]
    fn test_add_line_below_keeps_current_line_intact() {
        // Place cursor in the middle of "hello world" (e.g. offset 5)
        let mut state = State::from("hello world\nsecond line\n");
        let sel = Selection::cursor(5);
        state = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap().into_state();

        let res = add_line(&mut state, true);
        assert!(res.is_ok());
        // Original line "hello world" must NOT be broken
        assert_eq!(state.doc().to_string(), "hello world\n\nsecond line\n");
        // Cursor must be on the newly added line (line 1, offset 12)
        assert_eq!(state.selection().main().head(), 12);
        assert_eq!(state.doc().line_at(state.selection().main().head()).unwrap().number(), 1);
    }

    #[test]
    fn test_add_line_above_keeps_current_line_intact() {
        // Place cursor in the middle of "second line"
        let mut state = State::from("first line\nsecond line\n");
        let sel = Selection::cursor(15);
        state = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap().into_state();

        let res = add_line(&mut state, false);
        assert!(res.is_ok());
        // Original line must NOT be broken
        assert_eq!(state.doc().to_string(), "first line\n\nsecond line\n");
        // Cursor must be on the newly added line (line 1, offset 11)
        assert_eq!(state.selection().main().head(), 11);
        assert_eq!(state.doc().line_at(state.selection().main().head()).unwrap().number(), 1);
    }

    #[test]
    fn insert_hard_tab_replaces_selection_or_cursor() {
        let mut state = State::from("hello world");
        let sel = Selection::cursor(5);
        state = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap().into_state();

        let res = insert_hard_tab(&mut state);
        assert!(res.is_ok());
        assert_eq!(state.doc().to_string(), "hello\t world");
    }
}
