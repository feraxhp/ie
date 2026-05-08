use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, 
        Event, KeyCode, KeyModifiers, KeyEvent
    },
    execute,
    terminal::{ 
        disable_raw_mode, 
        enable_raw_mode,
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
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;


fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // let filename = "README.md";
    let filename = if args.len() > 1 { &args[1] } 
    else {
        eprintln!("Usage: ie <filename>");
        return Ok(());
    };
    
    let language = get_lang(filename);

    let (content, exist_) = match fs::exists(filename) {
        Ok(e) if e => (fs::read_to_string(filename)?, true),
        Ok(_) => (format!(""), false),
        Err(_) => todo!(),
    };

    let arc_exis: Arc<Mutex<bool>> = Arc::new(Mutex::new(exist_));
    let exist_clone = arc_exis.clone();
    
    let arc_unsave_lines: Arc<Mutex<HashSet<usize>>> = Arc::new(
        Mutex::new(HashSet::new())
    );
    
    enable_raw_mode()?;
    execute!(stdout(), EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(8),
        },
    )?;
    
    let theme = vesper();

    let mut editor = Editor::new(&language, &content, theme)?;
    let mut current_editor_area = ratatui::layout::Rect::default(); 

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
            
            // Renderizar editor
            f.render_widget(&editor, editor_area);
        
            if let Some((x, y)) = editor.get_visible_cursor(&editor_area) {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.code == KeyCode::Esc => break,
                Event::Key(key) if is_save_pressed(key) => {
                    let content = editor.get_content();
                    save_to_file(&content, filename)?;
                    let mut exist_local = exist_clone.lock().unwrap();
                    let mut unsave_lines = unsave_lines.lock().unwrap();
                    
                    *exist_local = true;
                    unsave_lines.clear();
                },
                Event::Key(key) => editor.input(key, &current_editor_area)?,
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &current_editor_area)?;
                },
                _ => {}
            }
        }
    }
    
    terminal.clear()?; 
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture
    )?;
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

// fn get_current_line(editor: &Editor, area: &Rect) -> Option<usize> {
//     let (_, vc) = editor.get_visible_cursor(area)?;
//     let top = area.top();
//     let current_line = (vc - top) as usize;
    
//     return Some(current_line);
// }
