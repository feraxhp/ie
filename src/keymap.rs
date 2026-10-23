use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mynox::Error;
use serde::{Deserialize, Serialize};
use texera::EditorAction;

use crate::system::{Directories, File};

/// All possible actions that can be triggered by a keybinding in `ie`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IeAction {
    // Editor and file control
    SaveFile,
    Exit,
    Abort,
    Escape,
    ToggleHelp,
    ToggleGutter,
    ToggleWrap,

    // Viewport actions (inline mode)
    IncreaseLines,
    DecreaseLines,

    // Search operations
    Search,
    NextMatch,
    PrevMatch,
    FocusEditor,
    FocusSearch,

    // Advanced editing actions
    CloneLineDown,
    CloneLineUp,
    MoveSelectionDown,
    MoveSelectionUp,
    DeleteLine,
    AddLineBelow,
    AddLineAbove,
    ToggleComments,

    // Core Texera editing/navigation actions
    Texera(EditorAction),
}

impl FromStr for IeAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "save-file" | "save" => Ok(IeAction::SaveFile),
            "exit" | "quit" => Ok(IeAction::Exit),
            "abort" | "cancel" => Ok(IeAction::Abort),
            "escape" | "esc" => Ok(IeAction::Escape),
            "toggle-help" | "toggle-hints" | "help" => Ok(IeAction::ToggleHelp),
            "toggle-gutter" => Ok(IeAction::ToggleGutter),
            "toggle-wrap" => Ok(IeAction::ToggleWrap),

            "increase-lines" | "add-line" | "increase-inline-lines" => Ok(IeAction::IncreaseLines),
            "decrease-lines" | "remove-line" | "decrease-inline-lines" => Ok(IeAction::DecreaseLines),

            "search" => Ok(IeAction::Search),
            "next-match" => Ok(IeAction::NextMatch),
            "prev-match" => Ok(IeAction::PrevMatch),
            "focus-editor" => Ok(IeAction::FocusEditor),
            "focus-search" => Ok(IeAction::FocusSearch),

            "clone-line-down" => Ok(IeAction::CloneLineDown),
            "clone-line-up" => Ok(IeAction::CloneLineUp),
            "move-selection-down" => Ok(IeAction::MoveSelectionDown),
            "move-selection-up" => Ok(IeAction::MoveSelectionUp),
            "delete-line" => Ok(IeAction::DeleteLine),
            "add-line-below" => Ok(IeAction::AddLineBelow),
            "add-line-above" => Ok(IeAction::AddLineAbove),
            "toggle-comments" | "toggle-coments" => Ok(IeAction::ToggleComments),

            // Texera actions
            "undo" => Ok(IeAction::Texera(EditorAction::Undo)),
            "redo" => Ok(IeAction::Texera(EditorAction::Redo)),
            "select-all" => Ok(IeAction::Texera(EditorAction::SelectAll)),
            "insert-newline" | "newline" | "enter" => Ok(IeAction::Texera(EditorAction::InsertNewline)),
            "insert-tab" | "tab" => Ok(IeAction::Texera(EditorAction::InsertTab)),
            "delete-backward" | "backspace" => Ok(IeAction::Texera(EditorAction::DeleteBackward)),
            "delete-forward" | "delete" => Ok(IeAction::Texera(EditorAction::DeleteForward)),
            "move-left" => Ok(IeAction::Texera(EditorAction::MoveLeft)),
            "move-right" => Ok(IeAction::Texera(EditorAction::MoveRight)),
            "move-up" => Ok(IeAction::Texera(EditorAction::MoveUp)),
            "move-down" => Ok(IeAction::Texera(EditorAction::MoveDown)),
            "select-left" => Ok(IeAction::Texera(EditorAction::SelectLeft)),
            "select-right" => Ok(IeAction::Texera(EditorAction::SelectRight)),
            "select-up" => Ok(IeAction::Texera(EditorAction::SelectUp)),
            "select-down" => Ok(IeAction::Texera(EditorAction::SelectDown)),
            "move-line-start" | "home" => Ok(IeAction::Texera(EditorAction::MoveLineStart)),
            "move-line-end" | "end" => Ok(IeAction::Texera(EditorAction::MoveLineEnd)),
            "select-line-start" => Ok(IeAction::Texera(EditorAction::SelectLineStart)),
            "select-line-end" => Ok(IeAction::Texera(EditorAction::SelectLineEnd)),
            "move-doc-start" => Ok(IeAction::Texera(EditorAction::MoveDocStart)),
            "move-doc-end" => Ok(IeAction::Texera(EditorAction::MoveDocEnd)),
            "add-cursor-above" => Ok(IeAction::Texera(EditorAction::AddCursorAbove)),
            "add-cursor-below" => Ok(IeAction::Texera(EditorAction::AddCursorBelow)),
            other => Err(format!("Unknown editor action: '{}'", other)),
        }
    }
}

/// Represents a parsed key and modifier combination.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Converts a crossterm KeyEvent to a normalized KeyCombo.
    pub fn from_event(event: &KeyEvent) -> Self {
        let mut modifiers = event.modifiers;
        // Normalize: crossterm sometimes reports Shift modifier with uppercase Char
        let code = match event.code {
            KeyCode::Char(c) if c.is_ascii_uppercase() => {
                modifiers.insert(KeyModifiers::SHIFT);
                KeyCode::Char(c.to_ascii_lowercase())
            }
            other => other,
        };

        // Filter only recognized modifier bits
        let clean_mods =
            modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER);

        Self {
            code,
            modifiers: clean_mods,
        }
    }
}

impl FromStr for KeyCombo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.is_empty() {
            return Err("Empty key combo".to_string());
        }

        let mut modifiers = KeyModifiers::empty();
        for part in &parts[..parts.len() - 1] {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "ctr" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" | "opt" | "option" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                "super" | "win" | "cmd" => modifiers.insert(KeyModifiers::SUPER),
                unknown => return Err(format!("Unknown modifier: '{}'", unknown)),
            }
        }

        let key_str = parts.last().unwrap().trim().to_ascii_lowercase();
        let code = match key_str.as_str() {
            "enter" | "return" => KeyCode::Enter,
            "backspace" => KeyCode::Backspace,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "esc" | "escape" => KeyCode::Esc,
            "space" => KeyCode::Char(' '),
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "slash" | "/" => KeyCode::Char('/'),
            "underscore" | "_" => KeyCode::Char('_'),
            s if s.starts_with('f') && s.len() > 1 => {
                let num = s[1..].parse::<u8>().map_err(|_| format!("Invalid function key: '{}'", s))?;
                KeyCode::F(num)
            }
            s if s.chars().count() == 1 => {
                KeyCode::Char(s.chars().next().unwrap())
            }
            unknown => return Err(format!("Unknown key code: '{}'", unknown)),
        };

        Ok(Self { code, modifiers })
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            write!(f, "ctrl-")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            write!(f, "alt-")?;
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            write!(f, "shift-")?;
        }
        if self.modifiers.contains(KeyModifiers::SUPER) {
            write!(f, "super-")?;
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "space"),
            KeyCode::Char(c) => write!(f, "{}", c),
            KeyCode::F(n) => write!(f, "f{}", n),
            KeyCode::Esc => write!(f, "esc"),
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Backspace => write!(f, "backspace"),
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::BackTab => write!(f, "backtab"),
            KeyCode::Delete => write!(f, "delete"),
            KeyCode::Insert => write!(f, "insert"),
            KeyCode::Left => write!(f, "left"),
            KeyCode::Right => write!(f, "right"),
            KeyCode::Up => write!(f, "up"),
            KeyCode::Down => write!(f, "down"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::PageUp => write!(f, "pageup"),
            KeyCode::PageDown => write!(f, "pagedown"),
            other => write!(f, "{:?}", other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeymapMode {
    Extend,
    Absolute,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeymapConfig {
    pub mode: KeymapMode,
    pub keys: HashMap<String, String>,
}

/// The active keymap for `ie`.
#[derive(Clone, Debug)]
pub struct IeKeymap {
    pub bindings: HashMap<KeyCombo, IeAction>,
}

impl Default for IeKeymap {
    fn default() -> Self {
        Self {
            bindings: Self::default_bindings(),
        }
    }
}

impl IeKeymap {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up the human-readable string representation of the first keycombo
    /// bound to the specified `action`. Returns `fallback` if not found.
    pub fn key_for_action(&self, action: IeAction, fallback: &str) -> String {
        for (combo, act) in &self.bindings {
            if *act == action {
                return combo.to_string();
            }
        }
        fallback.to_string()
    }

    /// Default keybindings defined for `ie``.
    pub fn default_bindings() -> HashMap<KeyCombo, IeAction> {
        let mut map = HashMap::new();

        // Standard editing
        map.insert(KeyCombo::new(KeyCode::Backspace, KeyModifiers::empty()), IeAction::Texera(EditorAction::DeleteBackward));
        map.insert(KeyCombo::new(KeyCode::Delete, KeyModifiers::empty()), IeAction::Texera(EditorAction::DeleteForward));
        map.insert(KeyCombo::new(KeyCode::Enter, KeyModifiers::empty()), IeAction::Texera(EditorAction::InsertNewline));
        map.insert(KeyCombo::new(KeyCode::Tab, KeyModifiers::empty()), IeAction::Texera(EditorAction::InsertTab));

        // Navigation
        map.insert(KeyCombo::new(KeyCode::Left, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveLeft));
        map.insert(KeyCombo::new(KeyCode::Right, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveRight));
        map.insert(KeyCombo::new(KeyCode::Up, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveUp));
        map.insert(KeyCombo::new(KeyCode::Down, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveDown));
        map.insert(KeyCombo::new(KeyCode::Home, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveLineStart));
        map.insert(KeyCombo::new(KeyCode::End, KeyModifiers::empty()), IeAction::Texera(EditorAction::MoveLineEnd));

        // Selection (with Shift)
        map.insert(KeyCombo::new(KeyCode::Left, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectLeft));
        map.insert(KeyCombo::new(KeyCode::Right, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectRight));
        map.insert(KeyCombo::new(KeyCode::Up, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectUp));
        map.insert(KeyCombo::new(KeyCode::Down, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectDown));
        map.insert(KeyCombo::new(KeyCode::Home, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectLineStart));
        map.insert(KeyCombo::new(KeyCode::End, KeyModifiers::SHIFT), IeAction::Texera(EditorAction::SelectLineEnd));

        // History
        map.insert(KeyCombo::new(KeyCode::Char('z'), KeyModifiers::CONTROL), IeAction::Texera(EditorAction::Undo));
        map.insert(KeyCombo::new(KeyCode::Char('y'), KeyModifiers::CONTROL), IeAction::Texera(EditorAction::Redo));
        map.insert(KeyCombo::new(KeyCode::Char('z'), KeyModifiers::CONTROL | KeyModifiers::SHIFT), IeAction::Texera(EditorAction::Redo));

        // Multi-cursor (Alt+Up / Alt+Down)
        map.insert(KeyCombo::new(KeyCode::Down, KeyModifiers::ALT), IeAction::Texera(EditorAction::AddCursorBelow));
        map.insert(KeyCombo::new(KeyCode::Up, KeyModifiers::ALT), IeAction::Texera(EditorAction::AddCursorAbove));

        // Selection shortcuts
        map.insert(KeyCombo::new(KeyCode::Char('a'), KeyModifiers::CONTROL), IeAction::Texera(EditorAction::SelectAll));

        // Application level
        map.insert(KeyCombo::new(KeyCode::Char('s'), KeyModifiers::CONTROL), IeAction::SaveFile);
        map.insert(KeyCombo::new(KeyCode::Char('c'), KeyModifiers::CONTROL), IeAction::Abort);
        map.insert(KeyCombo::new(KeyCode::Esc, KeyModifiers::empty()), IeAction::Escape);
        map.insert(KeyCombo::new(KeyCode::F(1), KeyModifiers::empty()), IeAction::ToggleHelp);
        map.insert(KeyCombo::new(KeyCode::F(2), KeyModifiers::empty()), IeAction::ToggleGutter);
        map.insert(KeyCombo::new(KeyCode::F(3), KeyModifiers::empty()), IeAction::ToggleWrap);
        map.insert(KeyCombo::new(KeyCode::F(5), KeyModifiers::empty()), IeAction::IncreaseLines);
        map.insert(KeyCombo::new(KeyCode::F(6), KeyModifiers::empty()), IeAction::DecreaseLines);

        // Requested specific features
        // Search dialog (ctrl-f / ctr-f)
        map.insert(KeyCombo::new(KeyCode::Char('f'), KeyModifiers::CONTROL), IeAction::Search);

        // Clone lines
        map.insert(KeyCombo::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT), IeAction::CloneLineDown);
        map.insert(KeyCombo::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT), IeAction::CloneLineUp);

        // Move selections
        map.insert(KeyCombo::new(KeyCode::Down, KeyModifiers::ALT | KeyModifiers::SHIFT), IeAction::MoveSelectionDown);
        map.insert(KeyCombo::new(KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT), IeAction::MoveSelectionUp);

        // Delete line
        map.insert(KeyCombo::new(KeyCode::Delete, KeyModifiers::SHIFT), IeAction::DeleteLine);
        map.insert(KeyCombo::new(KeyCode::Backspace, KeyModifiers::SHIFT), IeAction::DeleteLine);

        // Add line below / above
        map.insert(KeyCombo::new(KeyCode::Enter, KeyModifiers::SHIFT), IeAction::AddLineBelow);
        map.insert(KeyCombo::new(KeyCode::Enter, KeyModifiers::CONTROL), IeAction::AddLineAbove);

        // Toggle comments
        map.insert(KeyCombo::new(KeyCode::Char('c'), KeyModifiers::SHIFT), IeAction::ToggleComments);
        map.insert(KeyCombo::new(KeyCode::Char('k'), KeyModifiers::SHIFT), IeAction::ToggleComments);
        
        // Search mode navigation
        map.insert(KeyCombo::new(KeyCode::Char('n'), KeyModifiers::SHIFT), IeAction::NextMatch);
        map.insert(KeyCombo::new(KeyCode::Char('p'), KeyModifiers::SHIFT), IeAction::PrevMatch);
        map.insert(KeyCombo::new(KeyCode::Char('e'), KeyModifiers::SHIFT), IeAction::FocusEditor);
        map.insert(KeyCombo::new(KeyCode::Char('s'), KeyModifiers::SHIFT), IeAction::FocusSearch);

        map
    }

    /// Resolves an action for a given terminal keyboard event.
    pub fn action_for_key(&self, event: &KeyEvent) -> Option<IeAction> {
        let combo = KeyCombo::from_event(event);
        if let Some(action) = self.bindings.get(&combo).copied() {
            return Some(action);
        }

        // Fallback for regular character insertion when typing text without modifiers (or simple Shift for capital letters)
        let clean_mods = event.modifiers
            & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER);

        if clean_mods.is_empty() || clean_mods == KeyModifiers::SHIFT {
            if let KeyCode::Char(_) = event.code {
                return Some(IeAction::Texera(EditorAction::InsertChar));
            }
        }

        None
    }

    /// Loads custom keymap from the user configuration directory if `keymap.json` exists.
    pub fn load_or_default() -> Result<Self, Error> {
        let config_dir = match Directories::new() {
            Ok(pd) => pd.config_dir().to_path_buf(),
            Err(_) => return Ok(Self::default()),
        };

        let keymap_path = config_dir.join("keymap.json");
        if !keymap_path.exists() {
            return Ok(Self::default());
        }

        Self::load_from_path(&keymap_path)
    }

    /// Loads keymap from a specific file path.
    pub fn load_from_path(path: &Path) -> Result<Self, Error> {
        let content = File::read(path)?;
        Self::from_json_str(&content, path)
    }

    /// Parses a JSON configuration string and constructs an `IeKeymap`.
    pub fn from_json_str(json: &str, path_ref: &Path) -> Result<Self, Error> {
        let config: KeymapConfig = serde_json::from_str(json).map_err(|e| {
            mynox::error!(
                "config::keymap",
                "Invalid keymap configuration",
                format!("Failed to parse `{}`: {}", path_ref.display(), e)
                =>
                "Ensure keymap.json is valid JSON with `mode` ('extend' or 'absolute') and `keys` object"
            )
        })?;

        let mut bindings = match config.mode {
            KeymapMode::Extend => Self::default_bindings(),
            KeymapMode::Absolute => HashMap::new(),
        };

        for (key_str, action_str) in config.keys {
            let combo = KeyCombo::from_str(&key_str).map_err(|e| {
                mynox::error!(
                    "config::keymap",
                    "Invalid key combination",
                    format!("Error parsing key `{}` in `{}`: {}", key_str, path_ref.display(), e)
                )
            })?;

            let action = IeAction::from_str(&action_str).map_err(|e| {
                mynox::error!(
                    "config::keymap",
                    "Invalid action name",
                    format!("Error parsing action `{}` in `{}`: {}", action_str, path_ref.display(), e)
                )
            })?;

            bindings.insert(combo, action);
        }

        Ok(Self { bindings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_for_action() {
        let keymap = IeKeymap::default();
        assert_eq!(keymap.key_for_action(IeAction::Abort, "ctrl-c"), "ctrl-c");
        assert_eq!(keymap.key_for_action(IeAction::SaveFile, "ctrl-s"), "ctrl-s");
        assert_eq!(keymap.key_for_action(IeAction::Escape, "esc"), "esc");
    }

    #[test]
    fn test_shift_c_matches_toggle_comments() {
        let keymap = IeKeymap::default();

        // When crossterm delivers KeyCode::Char('c') with CONTROL | SHIFT
        let event2 = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SHIFT);
        assert_eq!(keymap.action_for_key(&event2), Some(IeAction::ToggleComments));
    }

    #[test]
    fn test_ctrl_c_maps_to_abort() {
        let keymap = IeKeymap::default();
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(keymap.action_for_key(&event), Some(IeAction::Abort));
    }

    #[test]
    fn test_escape_maps_to_escape() {
        let keymap = IeKeymap::default();
        let event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(keymap.action_for_key(&event), Some(IeAction::Escape));
    }

    #[test]
    fn test_f5_f6_default_bindings() {
        let keymap = IeKeymap::default();
        let f5_event = KeyEvent::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(keymap.action_for_key(&f5_event), Some(IeAction::IncreaseLines));
        let f6_event = KeyEvent::new(KeyCode::F(6), KeyModifiers::empty());
        assert_eq!(keymap.action_for_key(&f6_event), Some(IeAction::DecreaseLines));
    }

    #[test]
    fn test_parse_key_combo() {
        let combo = KeyCombo::from_str("ctrl-s").unwrap();
        assert_eq!(combo.code, KeyCode::Char('s'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));

        let combo = KeyCombo::from_str("ctr-f").unwrap();
        assert_eq!(combo.code, KeyCode::Char('f'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));

        let combo = KeyCombo::from_str("ctrl-shift-c").unwrap();
        assert_eq!(combo.code, KeyCode::Char('c'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT));

        let combo = KeyCombo::from_str("ctrl-shift-down").unwrap();
        assert_eq!(combo.code, KeyCode::Down);
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT));

        let combo = KeyCombo::from_str("alt-shift-up").unwrap();
        assert_eq!(combo.code, KeyCode::Up);
        assert!(combo.modifiers.contains(KeyModifiers::ALT | KeyModifiers::SHIFT));

        let combo = KeyCombo::from_str("shift-delete").unwrap();
        assert_eq!(combo.code, KeyCode::Delete);
        assert!(combo.modifiers.contains(KeyModifiers::SHIFT));

        let combo = KeyCombo::from_str("f2").unwrap();
        assert_eq!(combo.code, KeyCode::F(2));
        let combo = KeyCombo::from_str("f5").unwrap();
        assert_eq!(combo.code, KeyCode::F(5));
        let combo = KeyCombo::from_str("f6").unwrap();
        assert_eq!(combo.code, KeyCode::F(6));
    }

    #[test]
    fn test_parse_actions() {
        assert_eq!(IeAction::from_str("search").unwrap(), IeAction::Search);
        assert_eq!(IeAction::from_str("clone-line-down").unwrap(), IeAction::CloneLineDown);
        assert_eq!(IeAction::from_str("move-selection-down").unwrap(), IeAction::MoveSelectionDown);
        assert_eq!(IeAction::from_str("toggle-comments").unwrap(), IeAction::ToggleComments);
        assert_eq!(IeAction::from_str("toggle-coments").unwrap(), IeAction::ToggleComments);
        assert_eq!(IeAction::from_str("save-file").unwrap(), IeAction::SaveFile);
        assert_eq!(IeAction::from_str("exit").unwrap(), IeAction::Exit);
        assert_eq!(IeAction::from_str("quit").unwrap(), IeAction::Exit);
        assert_eq!(IeAction::from_str("abort").unwrap(), IeAction::Abort);
        assert_eq!(IeAction::from_str("cancel").unwrap(), IeAction::Abort);
        assert_eq!(IeAction::from_str("increase-lines").unwrap(), IeAction::IncreaseLines);
        assert_eq!(IeAction::from_str("add-line").unwrap(), IeAction::IncreaseLines);
        assert_eq!(IeAction::from_str("decrease-lines").unwrap(), IeAction::DecreaseLines);
        assert_eq!(IeAction::from_str("remove-line").unwrap(), IeAction::DecreaseLines);
    }

    #[test]
    fn test_keymap_mode_extend() {
        let json = r#"{"mode": "extend", "keys": {"ctrl-s": "save-file", "ctrl-k": "delete-line"}}"#;

        let keymap = IeKeymap::from_json_str(json, Path::new("test.json")).unwrap();
        // Preserves default bindings
        let f2_combo = KeyCombo::new(KeyCode::F(2), KeyModifiers::empty());
        assert_eq!(keymap.bindings.get(&f2_combo), Some(&IeAction::ToggleGutter));
        let f5_combo = KeyCombo::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(keymap.bindings.get(&f5_combo), Some(&IeAction::IncreaseLines));
        let f6_combo = KeyCombo::new(KeyCode::F(6), KeyModifiers::empty());
        assert_eq!(keymap.bindings.get(&f6_combo), Some(&IeAction::DecreaseLines));

        // Overrides or adds new keys
        let ctrl_k = KeyCombo::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(keymap.bindings.get(&ctrl_k), Some(&IeAction::DeleteLine));
    }

    #[test]
    fn test_keymap_mode_absolute() {
        let json = r#"{"mode": "absolute", "keys": {"ctrl-s": "save-file"}}"#;

        let keymap = IeKeymap::from_json_str(json, Path::new("test.json")).unwrap();
        assert_eq!(keymap.bindings.len(), 1);
        let ctrl_s = KeyCombo::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(keymap.bindings.get(&ctrl_s), Some(&IeAction::SaveFile));
        let f2_combo = KeyCombo::new(KeyCode::F(2), KeyModifiers::empty());
        assert_eq!(keymap.bindings.get(&f2_combo), None);
    }
}
