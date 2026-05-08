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
use ratatui::{Terminal, TerminalOptions, Viewport, backend::CrosstermBackend, layout::Position};
use std::{fs, io::stdout};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme::vesper;
use ratatui_code_editor::utils::get_lang;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    let filename = if args.len() > 1 { &args[1] } 
    else {
        eprintln!("Usage: et <filename>");
        return Ok(());
    };
    
    let language = get_lang(filename);

    let content = match fs::exists(filename) {
        Ok(e) if e => fs::read_to_string(filename)?,
        Ok(_) => format!(""),
        Err(_) => todo!(),
    };

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
    let mut editor_area = ratatui::layout::Rect::default(); 

    loop {

        terminal.draw(|f| {
            let area = f.area();
            editor_area = area;
            f.render_widget(&editor, area);
            
            let cursor = editor.get_visible_cursor(&area);
            if let Some((x,y)) = cursor {
                f.set_cursor_position(Position::new(x, y));
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Esc {
                        break;
                    } else if is_save_pressed(key) {
                        let content = editor.get_content();
                        save_to_file(&content, filename)?;
                    } else {
                        editor.input(key, &editor_area)?;
                    }
                }
                Event::Mouse(mouse) => {
                    editor.mouse(mouse, &editor_area)?;
                },
                Event::Resize(_, _) => { }
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