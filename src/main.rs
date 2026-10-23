mod cache;
mod command;
mod highlight;
mod permissions;
mod prompt;
mod system;

use std::io::{stdout, Write};
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};
use texera::{
    apply_action, DefaultKeymap, EditorAction, EditorView, EditorViewState, HistoryPlugin, Keymap,
    LineNumberPlugin, HighlightPlugin, UnsavedLinePlugin,
};
use texera_core::{AllowMultipleSelections, ChangeSpec, State, Text, TransactionSpec};

use crate::cache::Cache;
use crate::command::{get_options, TUImode};
use crate::highlight::{detect_language, init_highlighter};
use crate::permissions::{check_write_permission, save_file};
use crate::prompt::prompt_restore;
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
    if let Some(cached_text) = restored_content {
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

    let keymap = DefaultKeymap::new();
    let mut view_state = EditorViewState::new();
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
        &mut view_state,
        &highlight,
        &file_path,
        &mut permission_error,
        &mut first_line_row,
    );

    // 8. Clean exit and complete terminal restoration
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

fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut State,
    unsaved_lines: &mut UnsavedLinePlugin,
    keymap: &DefaultKeymap,
    view_state: &mut EditorViewState,
    highlight: &HighlightPlugin,
    file_path: &PathBuf,
    permission_error: &mut Option<String>,
    first_line_row: &mut Option<u16>,
) -> Result<(), mynox::Error> {
    let mut show_gutter = true;
    let mut wrap_enabled = false;

    loop {
        terminal
            .draw(|f| {
                let area = f.area();

                // Track the row of the first rendered line for clean inline exit
                if first_line_row.is_none() {
                    *first_line_row = Some(area.y);
                }

                // If write permissions failed, split vertically to render the red error bar below the editor
                let (editor_area, error_area) = if let Some(err_msg) = permission_error.as_deref() {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),    // Editor body
                            Constraint::Length(1), // Red warning bar
                        ])
                        .split(area);

                    (chunks[0], Some((chunks[1], err_msg)))
                } else {
                    (area, None)
                };

                // Base view with optional wrapping (wrap_enabled = false activates Texera's ultra-fast O(height) unwrapped path)
                let mut view = EditorView::new()
                    .with_style(Style::default())
                    .with_wrap(wrap_enabled)
                    .with_highlighter(highlight.clone());

                if show_gutter {
                    let gutter = LineNumberPlugin::new()
                        .with_style(Style::default().fg(Color::DarkGray))
                        .with_marker(unsaved_lines.clone());
                    view = view.with_gutter(gutter);
                }

                f.render_stateful_widget(view.bind(state), editor_area, view_state);

                // Render status bar underneath the editor explaining permission denial
                if let Some((err_rect, msg)) = error_area {
                    let error_bar = Paragraph::new(Line::from(vec![
                        Span::styled(
                            format!(" ✘ {}", msg),
                            Style::default()
                                .fg(Color::Red)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    f.render_widget(error_bar, err_rect);
                }
            })
            .map_err(|e| {
                mynox::error!(
                    "tui::draw",
                    "Render error",
                    format!("Failed to render user interface: {}", e)
                )
            })?;

        // Adaptive polling: poll at 8ms while the syntax parse is catching up with visible lines,
        // and 50ms once syntax is ready, minimizing CPU usage and stutter
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
                // Exit with Ctrl+C
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break;
                }

                // Handle Esc: collapse multiple selections down to one, or exit if already single
                if key_event.code == KeyCode::Esc {
                    match collapse_selections_or_exit(state) {
                        true => continue,
                        false => break,
                    }
                }

                // Toggle Line Gutter with F2
                if key_event.code == KeyCode::F(2) {
                    show_gutter = !show_gutter;
                    continue;
                }

                // Toggle Soft Line Wrapping with F3
                if key_event.code == KeyCode::F(3) {
                    wrap_enabled = !wrap_enabled;
                    continue;
                }

                // Save with Ctrl+S using write permission detector to avoid panicking
                if is_save_pressed(&key_event) {
                    let current_text = state.doc().to_string();
                    match save_file(&current_text, file_path) {
                        Ok(()) => {
                            unsaved_lines.mark_as_saved(state);
                            *permission_error = None;
                            let _ = Cache::remove(file_path);
                        }
                        Err(err) => {
                            // If saving failed due to permission denial, persist unsaved content to temp cache
                            if err.is_category("fs::permission_denied") {
                                let _ = Cache::save(file_path, &current_text);
                            }

                            // Display clean explanation without crashing or panicking
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

                // Dispatch editor actions through Texera
                if let Some(action) = keymap.action_for_key(&key_event) {
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
                            }
                        }
                        Err(_e) => {
                            // Non-fatal editor action error, continue loop
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn is_save_pressed(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s')
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

        // Add multiple cursors / selections
        let sel = Selection::from(vec![
            SelectionRange::cursor(0),
            SelectionRange::cursor(6),
            SelectionRange::cursor(12),
        ]);
        let update_res = state.clone().update(TransactionSpec::default().with_selection(sel)).unwrap();
        state = update_res.state().clone();
        assert_eq!(state.selection().count(), 3);

        // When multiple selections exist, collapse returns true and leaves exactly 1 selection
        let handled = collapse_selections_or_exit(&mut state);
        assert!(handled);
        assert_eq!(state.selection().count(), 1);

        // When only 1 selection exists, collapse returns false (signaling to exit)
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
}
