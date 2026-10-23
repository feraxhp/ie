mod cache;
mod command;
mod editor_actions;
mod highlight;
mod keymap;
mod permissions;
mod prompt;
mod search;
mod system;

use std::io::{stdout, Write};
use std::path::PathBuf;

use crossterm::event::{
    self, Event, KeyCode, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};
use texera::{
    apply_action, EditorAction, EditorView, EditorViewState, Gutter, HighlightPlugin,
    HistoryPlugin, LineNumberPlugin, UnsavedLinePlugin,
};
use texera_core::{AllowMultipleSelections, ChangeSpec, Selection, State, Text, TransactionSpec};

use crate::cache::Cache;
use crate::command::{get_options, TUImode};
use crate::editor_actions::{
    add_line, clone_lines, delete_lines, move_selection_lines, toggle_comments,
};
use crate::highlight::{detect_language, init_highlighter};
use crate::keymap::{IeAction, IeKeymap};
use crate::permissions::{check_write_permission, create_backup, save_file};
use crate::prompt::prompt_restore;
use crate::search::{SearchFocus, SearchState};
use crate::system::{Directories, File};

fn main() -> Result<(), mynox::Report> {
    if let Err(err) = run() {
        eprintln!("\r");
        return Err(mynox::Report::from(err));
    }
    Ok(())
}

fn run() -> Result<(), mynox::Error> {
    let options = get_options();
    let file_path = PathBuf::from(&options.file);

    // 1. Resolve or create user configuration directory for dynamic language specifications
    let languages_dir = Directories::languages_dir()?;

    // 2. Initialize HighlightPlugin with built-in languages and user config overrides
    let mut highlight = init_highlighter(&languages_dir);

    // Determine active syntax highlighting language (via CLI override or auto-detection)
    let active_lang = match options.format.as_deref() {
        Some(fmt) => Some(fmt.to_string()),
        None => detect_language(&file_path).map(|s| s.to_string()),
    };

    if let Some(ref lang) = active_lang {
        highlight.set_active_language(lang);
    }

    // 3. Read existing file or initialize with empty content using grp-style File reader
    let (content, file_existed) = if file_path.exists() {
        let text = File::read(&file_path)?;
        (text, true)
    } else {
        (String::new(), false)
    };

    // 4. Check if an unsaved edit was previously cached and write permissions are now available
    let has_cache = Cache::exists(&file_path);
    let has_write_perm = check_write_permission(&file_path).is_ok();

    let mut restored_content: Option<String> = None;
    if has_cache && has_write_perm {
        match prompt_restore()? {
            true => {
                let cached_text = Cache::read(&file_path)?;
                restored_content = Some(cached_text);
            }
            false => {
                let _ = Cache::remove(&file_path);
            }
        }
    }

    // 5. Configure Texera immutable transactional State with history, syntax cache, and multi-selection
    let mut state = State::from(content.as_str()).with_extensions([
        HistoryPlugin::extension(None),
        HighlightPlugin::syntax_extension(),
        AllowMultipleSelections(true).into(),
    ]);

    // 6. Configure editor plugins
    let mut unsaved_lines = UnsavedLinePlugin::new();
    if file_existed {
        unsaved_lines.mark_as_saved(&state);
    } else {
        // For new files that do not exist yet on disk, mark lines as unsaved
        unsaved_lines.saved_snapshot = Some(Vec::new());
        unsaved_lines.check_dirty_lines(&state);
    }

    // If restoring cached content, apply the cached edits over the original baseline buffer
    // so Myers diff properly activates dirty line markers
    if let Some(ref cached_text) = restored_content {
        let spec = TransactionSpec::default().with_change(
            ChangeSpec::new(
                0,
                Some(state.doc().len_chars()),
                Some(Text::from(cached_text.as_str())),
            )
            .map_err(|e| {
                mynox::error!(
                    "editor::restore",
                    "Failed to restore cached content",
                    format!("Could not apply cached text: {}", e)
                )
            })?,
        );

        if let Ok(result) = state.clone().update(spec) {
            state = result.into_state();
        }
        unsaved_lines.check_dirty_lines(&state);
    }

    // Jump to specified line/column if provided via CLI
    let mut initial_visual_line: Option<usize> = None;
    if options.line.is_some() || options.column.is_some() {
        let doc = state.doc();
        let num_lines = doc.len_lines();
        if num_lines > 0 {
            let target_line_idx = options
                .line
                .map(|l| l.max(1).saturating_sub(1).min(num_lines - 1))
                .unwrap_or(0);

            initial_visual_line = Some(target_line_idx);

            if let Ok(line) = doc.line(target_line_idx) {
                let line_text = line.text().to_string();
                let line_len_chars = line_text.trim_end_matches(['\r', '\n']).chars().count();
                let col_offset = options.column.map(|c| c.max(1).saturating_sub(1)).unwrap_or(0);
                let actual_col = col_offset.min(line_len_chars);
                let char_pos = line.from() + actual_col;

                let sel = Selection::cursor(char_pos);
                let spec = TransactionSpec::default().with_selection(sel);
                if let Ok(result) = state.clone().update(spec) {
                    state = result.into_state();
                }
            }
        }
    }

    // Load active keymap (from keymap.json in config dir or defaults)
    let keymap = IeKeymap::load_or_default()?;
    let mut search_state = SearchState::new();

    let mut view_state = EditorViewState::new();
    if let Some(target_line_idx) = initial_visual_line {
        let viewport_height: usize = match options.mode {
            TUImode::INLINE(lines) => lines as usize,
            TUImode::FULL => {
                crossterm::terminal::size().map(|(_, h)| h as usize).unwrap_or(24)
            }
        };
        let scroll_y = target_line_idx.saturating_sub(viewport_height / 2);
        view_state.set_scroll(0, scroll_y);
    }
    let mut permission_error: Option<String> = None;
    let mut first_line_row: Option<u16> = None;

    // 7. Initialize terminal according to selected mode (inline vs fullscreen)
    enable_raw_mode().map_err(|e| {
        mynox::error!(
            "tui::raw_mode",
            "Terminal initialization error",
            format!("Failed to enable **raw mode**: {}", e)
        )
    })?;

    // Enable progressive keyboard enhancement protocol if supported to disambiguate modified keys
    let _ = execute!(
        stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    );

    if matches!(options.mode, TUImode::FULL) {
        let mut out = stdout();
        execute!(out, EnterAlternateScreen).map_err(|e| {
            mynox::error!(
                "tui::screen",
                "Terminal initialization error",
                format!("Failed to enter **alternate screen**: {}", e)
            )
        })?;
    }

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = match options.mode {
        TUImode::INLINE(lines) => Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(lines),
            },
        )
        .map_err(|e| {
            mynox::error!(
                "tui::terminal",
                "Terminal initialization error",
                format!("Failed to initialize inline viewport with `{}` lines: {}", lines, e)
            )
        })?,
        TUImode::FULL => Terminal::new(backend).map_err(|e| {
            mynox::error!(
                "tui::terminal",
                "Terminal initialization error",
                format!("Failed to initialize fullscreen terminal: {}", e)
            )
        })?,
    };

    terminal.clear().map_err(|e| {
        mynox::error!(
            "tui::terminal",
            "Terminal initialization error",
            format!("Failed to clear terminal: {}", e)
        )
    })?;

    let res = run_event_loop(
        &mut terminal,
        &mut state,
        &mut unsaved_lines,
        &keymap,
        &mut search_state,
        &mut view_state,
        &highlight,
        &file_path,
        active_lang.as_deref(),
        options.backup,
        &mut permission_error,
        &mut first_line_row,
    );

    // 8. Clean exit and complete terminal restoration
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);

    // For inline mode: move cursor to the first line of the editor and clear everything below it
    if matches!(options.mode, TUImode::INLINE(_)) {
        if let Some(row) = first_line_row {
            let _ = execute!(
                terminal.backend_mut(),
                crossterm::cursor::MoveTo(0, row),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::FromCursorDown),
            );
        } else {
            let _ = terminal.clear();
        }
    } else {
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    }

    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();
    let _ = terminal.backend_mut().flush();
    eprint!("\r");

    res
}

enum BottomBarMode<'a> {
    Search,
    Error(&'a str),
}

fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut State,
    unsaved_lines: &mut UnsavedLinePlugin,
    keymap: &IeKeymap,
    search_state: &mut SearchState,
    view_state: &mut EditorViewState,
    highlight: &HighlightPlugin,
    file_path: &PathBuf,
    active_lang: Option<&str>,
    backup: bool,
    permission_error: &mut Option<String>,
    first_line_row: &mut Option<u16>,
) -> Result<(), mynox::Error> {
    let mut show_gutter = true;
    let mut wrap_enabled = false;
    let mut backup_done = false;

    loop {
        terminal
            .draw(|f| {
                let area = f.area();

                // Track the row of the first rendered line for clean inline exit
                if first_line_row.is_none() {
                    *first_line_row = Some(area.y);
                }

                // Split vertically when either search bar or write permission error bar is active
                let (editor_area, bottom_bar) = if search_state.is_active {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),    // Editor body
                            Constraint::Length(1), // Search bar
                        ])
                        .split(area);
                    (chunks[0], Some((chunks[1], BottomBarMode::Search)))
                } else if let Some(err_msg) = permission_error.as_deref() {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),    // Editor body
                            Constraint::Length(1), // Error bar
                        ])
                        .split(area);
                    (chunks[0], Some((chunks[1], BottomBarMode::Error(err_msg))))
                } else {
                    (area, None)
                };

                // Base view with optional wrapping
                let mut view = EditorView::new()
                    .with_style(Style::default())
                    .with_wrap(wrap_enabled)
                    .with_highlighter(highlight.clone());

                let gutter_width = if show_gutter {
                    let gutter = LineNumberPlugin::new()
                        .with_style(Style::default().fg(Color::DarkGray))
                        .with_marker(unsaved_lines.clone());
                    let width = gutter.width(state.doc().len_lines()).min(editor_area.width);
                    view = view.with_gutter(gutter);
                    width
                } else {
                    0
                };

                f.render_stateful_widget(view.bind(state), editor_area, view_state);

                // Render subtle search highlights onto buffer
                search_state.render_subtle_highlights(f.buffer_mut(), editor_area, state, view_state, gutter_width);

                // Render bottom bar (search or permission error)
                if let Some((bar_rect, mode)) = bottom_bar {
                    match mode {
                        BottomBarMode::Search => {
                            let (prompt_title, prompt_style) = if search_state.focus == SearchFocus::SearchInput {
                                (" search: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                            } else {
                                (" search [editor]: ", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))
                            };

                            let count_text = if search_state.query.is_empty() {
                                String::new()
                            } else if search_state.matches.is_empty() {
                                " (no match)".to_string()
                            } else {
                                format!(
                                    " ({}/{})",
                                    search_state.current_match_idx.map(|i| i + 1).unwrap_or(0),
                                    search_state.matches.len()
                                )
                            };

                            let hints = if search_state.focus == SearchFocus::SearchInput {
                                "  (shift-e: editor | shift-n: next | shift-p: prev | esc: close)"
                            } else {
                                "  (shift-s: search | shift-n: next | shift-p: prev | esc: close)"
                            };

                            let spans = if search_state.focus == SearchFocus::SearchInput {
                                let (before, after) = search_state.query_split_at_cursor();
                                vec![
                                    Span::styled(prompt_title, prompt_style),
                                    Span::styled(before, Style::default().fg(Color::White)),
                                    Span::styled(
                                        "_",
                                        Style::default()
                                            .fg(Color::Magenta)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(after, Style::default().fg(Color::White)),
                                    Span::styled(count_text, Style::default().fg(Color::Yellow)),
                                    Span::styled(hints, Style::default().fg(Color::DarkGray)),
                                ]
                            } else {
                                vec![
                                    Span::styled(prompt_title, prompt_style),
                                    Span::styled(&search_state.query, Style::default().fg(Color::White)),
                                    Span::styled(count_text, Style::default().fg(Color::Yellow)),
                                    Span::styled(hints, Style::default().fg(Color::DarkGray)),
                                ]
                            };

                            let bar = Paragraph::new(Line::from(spans));
                            f.render_widget(bar, bar_rect);
                        }
                        BottomBarMode::Error(msg) => {
                            let error_bar = Paragraph::new(Line::from(vec![
                                Span::styled(
                                    format!(" ✘ {}", msg),
                                    Style::default()
                                        .fg(Color::Red)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]));
                            f.render_widget(error_bar, bar_rect);
                        }
                    }
                }
            })
            .map_err(|e| {
                mynox::error!(
                    "tui::draw",
                    "Render error",
                    format!("Failed to render user interface: {}", e)
                )
            })?;

        // Adaptive polling
        let poll_timeout = if view_state.syntax_ready(state) {
            std::time::Duration::from_millis(50)
        } else {
            std::time::Duration::from_millis(8)
        };

        if event::poll(poll_timeout).map_err(|e| {
            mynox::error!(
                "tui::poll",
                "Event polling error",
                format!("Error while polling terminal events: {}", e)
            )
        })? {
            if let Event::Key(key_event) = event::read().map_err(|e| {
                mynox::error!(
                    "tui::event",
                    "Event read error",
                    format!("Failed to read keyboard event: {}", e)
                )
            })? {
                // Ignore key release and repeat events to avoid duplicate triggers
                if key_event.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }

                // If search mode is active and focused on search input
                if search_state.is_active && search_state.focus == SearchFocus::SearchInput {
                    // Check focus switch to editor (shift-e)
                    if key_event.modifiers.contains(KeyModifiers::SHIFT)
                        && (key_event.code == KeyCode::Char('e') || key_event.code == KeyCode::Char('E'))
                        || key_event.code == KeyCode::Char('E')
                    {
                        search_state.focus = SearchFocus::Editor;
                        continue;
                    }

                    // Check navigation within search mode (shift-n)
                    if key_event.modifiers.contains(KeyModifiers::SHIFT)
                        && (key_event.code == KeyCode::Char('n') || key_event.code == KeyCode::Char('N'))
                        || key_event.code == KeyCode::Char('N')
                    {
                        search_state.next_match(state);
                        continue;
                    }

                    // Check navigation within search mode (shift-p)
                    if key_event.modifiers.contains(KeyModifiers::SHIFT)
                        && (key_event.code == KeyCode::Char('p') || key_event.code == KeyCode::Char('P'))
                        || key_event.code == KeyCode::Char('P')
                    {
                        search_state.prev_match(state);
                        continue;
                    }

                    if key_event.code == KeyCode::Esc {
                        search_state.close();
                        continue;
                    }

                    if key_event.code == KeyCode::Enter {
                        search_state.next_match(state);
                        continue;
                    }

                    if key_event.code == KeyCode::Char('c') && key_event.modifiers == KeyModifiers::CONTROL {
                        break;
                    }

                    if key_event.code == KeyCode::Backspace {
                        if search_state.query_cursor > 0 {
                            search_state.query_cursor -= 1;
                            search_state.query.remove(search_state.query_cursor);
                            search_state.recompute_matches(state);
                            search_state.apply_current_match(state);
                        }
                        continue;
                    }

                    if key_event.code == KeyCode::Delete {
                        if search_state.query_cursor < search_state.query.len() {
                            search_state.query.remove(search_state.query_cursor);
                            search_state.recompute_matches(state);
                            search_state.apply_current_match(state);
                        }
                        continue;
                    }

                    if key_event.code == KeyCode::Left {
                        search_state.query_cursor = search_state.query_cursor.saturating_sub(1);
                        continue;
                    }

                    if key_event.code == KeyCode::Right {
                        search_state.query_cursor = (search_state.query_cursor + 1).min(search_state.query.len());
                        continue;
                    }

                    if let KeyCode::Char(c) = key_event.code {
                        if !key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && !key_event.modifiers.contains(KeyModifiers::ALT)
                        {
                            search_state.query.insert(search_state.query_cursor, c);
                            search_state.query_cursor += 1;
                            search_state.recompute_matches(state);
                            search_state.apply_current_match(state);
                            continue;
                        }
                    }

                    continue;
                }

                // Key dispatch when focus is in editor
                if let Some(action) = keymap.action_for_key(&key_event) {
                    match action {
                        IeAction::Exit => break,
                        IeAction::Escape => {
                            if search_state.is_active {
                                search_state.close();
                                continue;
                            }
                            match collapse_selections_or_exit(state) {
                                true => continue,
                                false => break,
                            }
                        }
                        IeAction::FocusSearch => {
                            if search_state.is_active {
                                search_state.focus = SearchFocus::SearchInput;
                                continue;
                            }
                        }
                        IeAction::FocusEditor => {
                            // Already focused on editor
                            continue;
                        }
                        IeAction::NextMatch => {
                            if search_state.is_active {
                                search_state.next_match(state);
                                continue;
                            }
                        }
                        IeAction::PrevMatch => {
                            if search_state.is_active {
                                search_state.prev_match(state);
                                continue;
                            }
                        }
                        IeAction::Search => {
                            search_state.open(state);
                            continue;
                        }
                        IeAction::ToggleGutter => {
                            show_gutter = !show_gutter;
                            continue;
                        }
                        IeAction::ToggleWrap => {
                            wrap_enabled = !wrap_enabled;
                            continue;
                        }
                        IeAction::SaveFile => {
                            let current_text = state.doc().to_string();

                            let backup_step = if backup && !backup_done {
                                create_backup(file_path)
                            } else {
                                Ok(None)
                            };

                            let save_result = match backup_step {
                                Ok(_) => {
                                    backup_done = true;
                                    save_file(&current_text, file_path)
                                }
                                Err(err) => Err(err),
                            };

                            match save_result {
                                Ok(()) => {
                                    unsaved_lines.mark_as_saved(state);
                                    *permission_error = None;
                                    let _ = Cache::remove(file_path);
                                }
                                Err(err) => {
                                    if err.is_category("fs::permission_denied") {
                                        let _ = Cache::save(file_path, &current_text);
                                    }
                                    let description = if err.is_category("fs::permission_denied") {
                                        format!(
                                            "Permission denied: You do not have write permissions to modify `{}`",
                                            file_path.display()
                                        )
                                    } else {
                                        format!(
                                            "Permission denied: Unable to modify `{}` ({})",
                                            file_path.display(),
                                            err.message()
                                        )
                                    };
                                    *permission_error = Some(description);
                                }
                            }
                            continue;
                        }
                        IeAction::CloneLineDown => {
                            if clone_lines(state, true)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::CloneLineUp => {
                            if clone_lines(state, false)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::MoveSelectionDown => {
                            if move_selection_lines(state, true)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::MoveSelectionUp => {
                            if move_selection_lines(state, false)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::DeleteLine => {
                            if delete_lines(state)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::AddLineBelow => {
                            if add_line(state, true)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::AddLineAbove => {
                            if add_line(state, false)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::ToggleComments => {
                            if toggle_comments(state, active_lang)? {
                                unsaved_lines.check_dirty_lines(state);
                                if search_state.is_active {
                                    search_state.recompute_matches(state);
                                }
                            }
                            continue;
                        }
                        IeAction::Texera(action) => {
                            let is_edit = matches!(
                                action,
                                EditorAction::InsertChar
                                    | EditorAction::InsertNewline
                                    | EditorAction::InsertTab
                                    | EditorAction::DeleteBackward
                                    | EditorAction::DeleteForward
                                    | EditorAction::Undo
                                    | EditorAction::Redo
                            );

                            match apply_action(state, action, Some(&key_event)) {
                                Ok(changed) => {
                                    if changed && is_edit {
                                        unsaved_lines.check_dirty_lines(state);
                                        if search_state.is_active {
                                            search_state.recompute_matches(state);
                                        }
                                    }
                                }
                                Err(_e) => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Collapses multi-cursor selections to a single selection, or returns false if already single.
pub fn collapse_selections_or_exit(state: &mut State) -> bool {
    if state.selection().count() > 1 {
        let single_sel = state.selection().clone().as_single();
        if let Ok(result) = state.clone().update(TransactionSpec::default().with_selection(single_sel)) {
            *state = result.state().clone();
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use texera_core::{Selection, SelectionRange};

    #[test]
    fn test_collapse_multiple_selections_to_single_on_escape() {
        let mut state = State::from("hello world\nsecond line\nthird line").with_extensions([
            AllowMultipleSelections(true).into(),
        ]);

        let sel = Selection::from(vec![
            SelectionRange::cursor(0),
            SelectionRange::cursor(6),
            SelectionRange::cursor(12),
        ]);
        let update_res = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap();
        state = update_res.state().clone();
        assert_eq!(state.selection().count(), 3);

        let handled = collapse_selections_or_exit(&mut state);
        assert!(handled);
        assert_eq!(state.selection().count(), 1);

        let handled_second = collapse_selections_or_exit(&mut state);
        assert!(!handled_second);
        assert_eq!(state.selection().count(), 1);
    }

    #[test]
    fn test_restore_cached_content_activates_dirty_lines() {
        let original = "line 1\nline 2\nline 3\n";
        let cached = "line 1\nline 2 edited\nline 3\nline 4 added\n";

        let mut state = State::from(original).with_extensions([
            AllowMultipleSelections(true).into(),
        ]);
        let mut unsaved_lines = UnsavedLinePlugin::new();
        unsaved_lines.mark_as_saved(&state);
        assert!(!unsaved_lines.is_dirty());

        let spec = TransactionSpec::default().with_change(
            ChangeSpec::new(0, Some(state.doc().len_chars()), Some(Text::from(cached))).unwrap(),
        );
        let update_res = state.clone().update(spec).unwrap();
        state = update_res.into_state();

        unsaved_lines.check_dirty_lines(&state);
        assert!(unsaved_lines.is_dirty());
        assert!(unsaved_lines.is_line_dirty(2));
        assert!(unsaved_lines.is_line_dirty(4));
    }

    #[test]
    fn test_jump_to_line_and_column() {
        let content = "line 1\nline 2 with more text\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
        let mut state = State::from(content);
        let doc = state.doc();
        let num_lines = doc.len_lines();
        assert_eq!(num_lines, 11);

        // Jump to line 2, col 8
        let target_line_idx = 2usize.saturating_sub(1); // line 2 -> index 1
        let line = doc.line(target_line_idx).unwrap();
        let col_offset = 8usize.saturating_sub(1); // col 8 -> offset 7
        let char_pos = line.from() + col_offset;

        let sel = Selection::cursor(char_pos);
        state = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap().into_state();

        assert_eq!(state.selection().main().head(), char_pos);
        assert_eq!(state.doc().line_at(state.selection().main().head()).unwrap().number(), 1);

        // Test viewport centering calculation
        let viewport_height = 8usize;
        let scroll_y = target_line_idx.saturating_sub(viewport_height / 2);
        assert_eq!(scroll_y, 0);

        // Jump to line 8 (index 7)
        let target_line_idx = 7usize;
        let scroll_y = target_line_idx.saturating_sub(viewport_height / 2);
        assert_eq!(scroll_y, 3); // 7 - 4 = 3
    }
}
