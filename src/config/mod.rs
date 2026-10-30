#[cfg(test)]
mod test;

use std::path::Path;

use texera::WhitespacePlugin;
use ratatui::style::Color;
use ratatui::style::Style;
use serde::Deserializer;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use mynox::Error;

use crate::system::Directories;
use crate::system::File;

fn deserialize_char_opt<'de, D>(deserializer: D) -> Result<Option<char>, D::Error>
where 
    D: Deserializer<'de>,
{
    let opt: Option<Value> = Deserialize::deserialize(deserializer)?;
    match opt {
        Some(Value::String(s)) => Ok(s.chars().next()),
        Some(_) => Err(serde::de::Error::custom("expected a character or string")),
        None => Ok(None),
    }
}

/// Whitespace character visualization settings.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct WhitespaceConfig {
    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub space: Option<char>,

    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub tab_filler: Option<char>,

    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub tab_terminator: Option<char>,
}

/// Global editor configuration for `ie`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub whitespace: Option<WhitespaceConfig>,

    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub space: Option<char>,

    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub tab_filler: Option<char>,

    #[serde(default, deserialize_with = "deserialize_char_opt")]
    pub tab_terminator: Option<char>,
}

impl Config {
    /// Loads custom editor configuration from `~/.config/ie/config.json` if it exists.
    pub fn load_or_default() -> Self {
        let config_dir = match Directories::new() {
            Ok(pd) => pd.config_dir().to_path_buf(),
            Err(_) => return Self::default(),
        };

        let config_path = config_dir.join("config.json");
        if !config_path.exists() {
            return Self::default();
        }

        Self::load_from_path(&config_path).unwrap_or_default()
    }

    /// Loads config from a specific path.
    pub fn load_from_path(path: &Path) -> Result<Self, Error> {
        let content = File::read(path)?;
        Self::from_json_str(&content, path)
    }

    /// Parses a JSON configuration string into `Config`.
    pub fn from_json_str(json: &str, path_ref: &Path) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(|e| {
            mynox::error!(
                "config::load",
                "Invalid editor configuration",
                format!("Failed to parse `{}`: {}", path_ref.display(), e)
                =>
                "Ensure config.json contains valid JSON configuration"
            )
        })
    }

    /// Constructs a `WhitespacePlugin` instance configured with glyphs.
    pub fn whitespace_plugin(&self) -> WhitespacePlugin {
        let mut plugin = WhitespacePlugin::default()
            .with_style(Style::default().fg(Color::DarkGray));

        let space = self.whitespace.as_ref().and_then(|w| w.space).or(self.space);
        let tab_filler = self.whitespace.as_ref().and_then(|w| w.tab_filler).or(self.tab_filler);
        let tab_terminator = self.whitespace.as_ref().and_then(|w| w.tab_terminator).or(self.tab_terminator);

        if let Some(s) = space {
            plugin = plugin.with_space_char(s);
        }
        if let Some(f) = tab_filler {
            plugin = plugin.with_tab_filler(f);
        }
        if let Some(t) = tab_terminator {
            plugin = plugin.with_tab_terminator(t);
        }

        plugin
    }
}
