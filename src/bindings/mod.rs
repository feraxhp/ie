use crossterm::event::{KeyEvent, KeyCode};
use ratatui_code_editor::editor::Editor;

pub fn keymap(editor: &mut Editor, key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::End => {
            let cursor = editor.get_cursor();
            let code = editor.code_ref();
            
            let line_idx = code.char_to_line(cursor);
            let start = code.line_to_char(line_idx);
            let end = start + code.line_len(line_idx);
            
            editor.set_cursor(end);
            true
        }
        KeyCode::Home => {
            let cursor = editor.get_cursor();
            let code = editor.code_ref();
            
            let line_idx = code.char_to_line(cursor);
            let start = code.line_to_char(line_idx);
            
            editor.set_cursor(start);
            true
        }
        _ => false, 
    }
}