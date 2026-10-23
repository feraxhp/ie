use std::io::{stdout, IsTerminal, Write};
use crossterm::cursor::{Hide, MoveToColumn, MoveUp, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use mynox::Error;

/// Prompts the user interactively before starting the editor whether to restore
/// the previously unsaved cached edition.
///
/// Layout:
/// do you whant to restore the last edition to that file?
/// > yes    no       (when yes is selected, in green)
///   yes  > no       (when no is selected, in red)
pub fn prompt_restore() -> Result<bool, Error> {
    if !stdout().is_terminal() {
        return Ok(false);
    }

    enable_raw_mode().map_err(|e| {
        mynox::error!(
            "tui::raw_mode",
            "Terminal initialization error",
            format!("Failed to enable raw mode for restore prompt: {}", e)
        )
    })?;

    let mut out = stdout();
    let _ = execute!(out, Hide);

    let mut selected_yes = true;

    // Line 1: Question
    let _ = write!(out, "do you whant to restore the last edition to that file?\r\n");
    // Line 2: Options
    render_options(&mut out, selected_yes);
    let _ = out.flush();

    let result = loop {
        match event::read() {
            Ok(Event::Key(KeyEvent { code, modifiers, .. })) => {
                if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                    break Ok(false);
                }
                match code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        selected_yes = !selected_yes;
                        render_options(&mut out, selected_yes);
                        let _ = out.flush();
                    }
                    KeyCode::Enter => {
                        break Ok(selected_yes);
                    }
                    KeyCode::Esc => {
                        break Ok(false);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                break Err(mynox::error!(
                    "tui::event",
                    "Event read error",
                    format!("Failed to read keyboard event: {}", e)
                ));
            }
            _ => {}
        }
    };

    // Clean up prompt lines from terminal to preserve seamless viewport initialization
    let _ = execute!(
        out,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        MoveUp(1),
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
        Show
    );
    let _ = out.flush();
    let _ = disable_raw_mode();

    result
}

pub fn render_options<W: Write>(out: &mut W, selected_yes: bool) {
    let _ = execute!(out, MoveToColumn(0), Clear(ClearType::CurrentLine));
    if selected_yes {
        // > yes (green)    no 
        let _ = execute!(out, SetForegroundColor(Color::Green), SetAttribute(Attribute::Bold));
        let _ = write!(out, "> yes");
        let _ = execute!(out, ResetColor, SetAttribute(Attribute::Reset));
        let _ = write!(out, "    no \r");
    } else {
        //   yes  > no (red)
        let _ = write!(out, "  yes  ");
        let _ = execute!(out, SetForegroundColor(Color::Red), SetAttribute(Attribute::Bold));
        let _ = write!(out, "> no ");
        let _ = execute!(out, ResetColor, SetAttribute(Attribute::Reset));
        let _ = write!(out, "\r");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_options_yes_selected() {
        let mut buf = Vec::new();
        render_options(&mut buf, true);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("> yes"));
        assert!(output.contains("    no "));
    }

    #[test]
    fn test_render_options_no_selected() {
        let mut buf = Vec::new();
        render_options(&mut buf, false);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("  yes  "));
        assert!(output.contains("> no "));
    }
}
