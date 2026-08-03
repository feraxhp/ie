mod command;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers
    },
    execute,
    terminal::{ 
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode
    },
};
use ratatui::{
    Terminal, TerminalOptions, Viewport, backend::CrosstermBackend, layout::{
        Constraint, 
        Direction, 
        Layout, 
        Position,
    }, style::{
        Color, 
        Style
    }, text::{
        Line, 
        Span
    }, widgets::Paragraph
};
use std::{cmp::min, collections::HashSet, fs, io::stdout, sync::{Arc, Mutex}};
use git2::Repository;
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;

use crate::command::{TUImode, get_options};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditMode {
    Plain,
    Diff,
    Combine,
}

impl EditMode {
    fn next(self) -> Self {
        match self {
            EditMode::Plain => EditMode::Diff,
            EditMode::Diff => EditMode::Combine,
            EditMode::Combine => EditMode::Plain,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let options = get_options();
    let language = match options.format {
        Some(f) => f,
        None => get_lang(&options.file),
    };

    let (mut content, exist_) = match fs::exists(&options.file) {
        Ok(e) if e => (fs::read_to_string(&options.file)?, true),
        Ok(_) => (format!(""), false),
        Err(_) => todo!(),
    };

    let arc_exis: Arc<Mutex<bool>> = Arc::new(Mutex::new(exist_));
    let exist_clone = arc_exis.clone();
    
    let arc_unsave_lines: Arc<Mutex<HashSet<usize>>> = Arc::new(
        Mutex::new(HashSet::new())
    );
    
    enable_raw_mode()?;
    if matches!(options.mode, TUImode::FULL) { 
        execute!(stdout(), EnableMouseCapture)?;
        execute!(stdout(), EnterAlternateScreen)?; 
    }
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = match &options.mode {
        command::TUImode::INLINE(lines) => Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(lines.clone()),
            },
        )?,
        command::TUImode::FULL => Terminal::new(backend)?,
    };

    terminal.clear()?;
    
    let theme = vesper();

    let mut editor = Editor::new(&language, &content, theme)?;
    let mut current_editor_area = ratatui::layout::Rect::default(); 

    // Initialize diff mode
    let mut edit_mode = EditMode::Combine;
    apply_mode(&mut editor, edit_mode);
    if let Some(original) = read_file_from_head(&options.file)? {
        editor.set_original_code(&original)?;
    }

    let us = arc_unsave_lines.clone();
    let exist_clone2 = arc_exis.clone();
    editor.set_change_callback(Box::new(move |changes| {
        let mut unsave_lines = us.lock().unwrap();
        let exist_local = exist_clone2.lock().unwrap();

        if *exist_local {
            for change in changes {
                _ = unsave_lines.insert(change.0)
            }
        }
    }));

    let unsave_lines = arc_unsave_lines.clone();
    
    loop {
        terminal.draw(|f| {
            let area = f.area();
            
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(1), // Status bar
                    Constraint::Min(0),    // Editor
                ])
                .split(area);
        
            let status_line_area = chunks[0];
            let editor_area = chunks[1];
            
            current_editor_area = editor_area;

            let unsave_lines = unsave_lines.lock().unwrap();
            let exist_local = exist_clone.lock().unwrap();
            let mut lines = Vec::new();
            for line in 0..min(status_line_area.height as usize, editor.code_ref().len_lines()) {
                let offset = editor.get_offset_y();
                let current_line = offset + line;
                
                let (char, color) = match *exist_local {
                    true if unsave_lines.contains(&current_line) => ("▎", Color::LightYellow),
                    true => (" ", Color::Green),
                    false => ("▎", Color::Red),
                };

                lines.push(Line::from(Span::styled(
                    char, 
                    Style::default().fg(color)
                )));
            }

            // Render Status bar
            let slim_bar = Paragraph::new(lines);
            f.render_widget(slim_bar, status_line_area);
            
            // Render editor
            f.render_widget(&editor, editor_area);
        
            if let Some((x, y)) = editor.get_visible_cursor(&editor_area) {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.code == KeyCode::Esc => break,
                Event::Key(key) if is_save_pressed(key) => {
                    let real_content = editor.get_content();
                    save_to_file(&real_content, &options.file)?;
                    let mut exist_local = exist_clone.lock().unwrap();
                    let mut unsave_lines = unsave_lines.lock().unwrap();
                    content = real_content;
                    
                    *exist_local = true;
                    unsave_lines.clear();
                },
                Event::Key(key) if is_cycle_edit_mode_pressed(key) => {
                    edit_mode = edit_mode.next();
                    apply_mode(&mut editor, edit_mode);
                    editor.focus(&current_editor_area);
                },
                Event::Key(key) => {
                    editor.input(key, &current_editor_area)?;
                    let exist_local = exist_clone.lock().unwrap();

                    if *exist_local && content == editor.get_content() {
                        let mut unsave_lines = unsave_lines.lock().unwrap();
                        unsave_lines.clear();
                    }
                },
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &current_editor_area)?;
                },
                _ => {}
            }
        }
    }
    
    terminal.clear()?; 
    disable_raw_mode()?;
    
    if matches!(options.mode, TUImode::FULL) { 
        execute!(
            terminal.backend_mut(),
            DisableMouseCapture
        )?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?; 
    }
    eprint!("\r");
    Ok(())
}

fn save_to_file(content: &str, path: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn is_save_pressed(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) &&
        key.code == KeyCode::Char('s')
}

fn is_cycle_edit_mode_pressed(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f')
}

fn apply_mode(editor: &mut Editor, mode: EditMode) {
    match mode {
        EditMode::Plain => {
            editor.set_diff_enabled(false);
            editor.set_diff_focus_enabled(false);
        }
        EditMode::Diff => {
            editor.set_diff_enabled(true);
            editor.set_diff_focus_context(3);
            editor.set_diff_focus_enabled(true);
        }
        EditMode::Combine => {
            editor.set_diff_enabled(true);
            editor.set_diff_focus_enabled(false);
        }
    }
}

fn read_file_from_head(path: &str) -> anyhow::Result<Option<String>> {
    let try_read = || -> anyhow::Result<String> {
        let abs_path = std::fs::canonicalize(path)?;
        let repo = Repository::discover(&abs_path)?;
        let workdir = repo.workdir().ok_or_else(|| anyhow::anyhow!("bare repo"))?;
        let rel_path = abs_path.strip_prefix(workdir)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let tree = commit.tree()?;
        let entry = tree.get_path(rel_path)?;
        let object = entry.to_object(&repo)?;
        let blob = object
            .as_blob()
            .ok_or_else(|| anyhow::anyhow!("not a blob"))?;
        let text = std::str::from_utf8(blob.content())?.to_string();
        Ok(text)
    };

    Ok(try_read().ok())
}
