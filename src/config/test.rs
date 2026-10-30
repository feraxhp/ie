use std::path::Path;

use ratatui::style::Color;
use ratatui::style::Style;
use mynox::Error;

use super::Config;

#[test]
fn default_config_produces_default_glyphs() {
    let config = Config::default();
    let plugin = config.whitespace_plugin();

    assert_eq!(plugin.space_char, '·');
    assert_eq!(plugin.tab_filler, ' ');
    assert_eq!(plugin.tab_terminator, '⇥');
    assert_eq!(plugin.style, Some(Style::default().fg(Color::DarkGray)));
}

#[test]
fn parses_nested_whitespace_config() -> Result<(), Error> {
    let json = r#"{
        "whitespace": {
            "space": ".",
            "tab_filler": "-",
            "tab_terminator": ">"
        }
    }"#;

    let config = Config::from_json_str(json, Path::new("config.json"))?;
    let plugin = config.whitespace_plugin();

    assert_eq!(plugin.space_char, '.');
    assert_eq!(plugin.tab_filler, '-');
    assert_eq!(plugin.tab_terminator, '>');

    Ok(())
}

#[test]
fn parses_root_level_whitespace_config() -> Result<(), Error> {
    let json = r#"{
        "space": "_",
        "tab_filler": " ",
        "tab_terminator": "→"
    }"#;

    let config = Config::from_json_str(json, Path::new("config.json"))?;
    let plugin = config.whitespace_plugin();

    assert_eq!(plugin.space_char, '_');
    assert_eq!(plugin.tab_filler, ' ');
    assert_eq!(plugin.tab_terminator, '→');

    Ok(())
}

#[test]
fn nested_whitespace_precedes_root_level() -> Result<(), Error> {
    let json = r#"{
        "space": "_",
        "whitespace": {
            "space": "•"
        }
    }"#;

    let config = Config::from_json_str(json, Path::new("config.json"))?;
    let plugin = config.whitespace_plugin();

    assert_eq!(plugin.space_char, '•');

    Ok(())
}

#[test]
fn handles_invalid_json_gracefully() {
    let json = "{ invalid json }";
    let res = Config::from_json_str(json, Path::new("config.json"));

    assert!(res.is_err());
}
