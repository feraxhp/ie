mod command;
mod highlight;
mod permissions;
mod system;

use std::fs;
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
use texera_core::{AllowMultipleSelections, State};

use crate::command::{get_options, TUImode};
use crate::highlight::{detect_language, init_highlighter};
use crate::permissions::save_file;
use crate::system::Directories;

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

    // 3. Read existing file or initialize with empty content
    let (content, file_existed) = if file_path.exists() {
        let text = fs::read_to_string(&file_path).map_err(|e| {
            mynox::error!(
                "fs::read",
                "Failed to read file",
                format!("Could not read `{}`: {}", file_path.display(), e)
                =>
                "Ensure the file exists and you have **read permissions**"
            )
        })?;
        (text, true)
    } else {
        (String::new(), false)
    };

    // 4. Configure Texera immutable transactional State with history, syntax cache, and multi-selection
    let mut state = State::from(content.as_str()).with_extensions([
        HistoryPlugin::extension(None),
        HighlightPlugin::syntax_extension(),
        AllowMultipleSelections(true).into(),
    ]);

    // 5. Configure editor plugins
    let mut unsaved_lines = UnsavedLinePlugin::new();
    if file_existed {
        unsaved_lines.mark_as_saved(&state);
    } else {
        // For new files that do not exist yet on disk, mark lines as unsaved
        unsaved_lines.saved_snapshot = Some(Vec::new());
        unsaved_lines.check_dirty_lines(&state);
    }

    let keymap = DefaultKeymap::new();
    let mut view_state = EditorViewState::new();
    let mut permission_error: Option<String> = None;
    let mut first_line_row: Option<u16> = None;

    // 6. Initialize terminal according to selected mode (inline vs fullscreen)
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

    // 7. Clean exit and complete terminal restoration
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
                // Exit with Esc or Ctrl+C
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
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
                        }
                        Err(err) => {
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
