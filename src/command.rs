use std::ffi::OsString;
use std::path::Path;
use clap::{Command, ValueHint, arg, value_parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TUImode {
    INLINE(u16),
    FULL,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub mode: TUImode,
    pub file: String,
    pub format: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub backup: bool,
}

pub fn build_command() -> Command {
    Command::new("ie")
        .about("A fast inline and fullscreen TUI editor")
        .args([
            arg!( <file> "The path of the file to be opened (supports 'file:line[:col]' or '+line[:col]')" )
                .value_hint(ValueHint::FilePath),
            arg!( --full "Open the editor in fullscreen TUI mode" ),
            arg!( -l --lines [number] "The number of lines to display (inline mode only)" )
                .conflicts_with("full")
                .default_value("8")
                .value_parser(value_parser!(u16)),
            arg!( -f --format [format] "Overrides the detected language for syntax highlighting" ),
            arg!( -b --backup "Create a backup file (<file>.bak) before the first save" ),
        ])
}

/// Parses a string formatted as "line" or "line:col" into 1-based (line, Option<col>).
pub fn parse_line_col_str(s: &str) -> Option<(usize, Option<usize>)> {
    if s.is_empty() {
        return None;
    }
    if let Some((line_str, col_str)) = s.split_once(':') {
        let line = line_str.parse::<usize>().ok()?;
        let col = col_str.parse::<usize>().ok()?;
        Some((line.max(1), Some(col.max(1))))
    } else {
        let line = s.parse::<usize>().ok()?;
        Some((line.max(1), None))
    }
}

/// Parses a raw file argument. If a file exists literally on disk at `raw`, it returns it as-is.
/// Otherwise, checks for trailing `:<line>` or `:<line>:<col>`.
pub fn parse_file_and_location(raw: &str) -> (String, Option<usize>, Option<usize>) {
    // If the path literally exists as a file on disk, do not split it.
    if Path::new(raw).exists() {
        return (raw.to_string(), None, None);
    }

    if let Some(last_colon) = raw.rfind(':') {
        let suffix = &raw[last_colon + 1..];
        let prefix = &raw[..last_colon];

        if let Ok(last_num) = suffix.parse::<usize>() {
            // Check if there is a preceding colon for `file:line:col`
            if let Some(prev_colon) = prefix.rfind(':') {
                let mid = &prefix[prev_colon + 1..];
                let file_prefix = &prefix[..prev_colon];
                if let Ok(line_num) = mid.parse::<usize>() {
                    if !file_prefix.is_empty() {
                        return (
                            file_prefix.to_string(),
                            Some(line_num.max(1)),
                            Some(last_num.max(1)),
                        );
                    }
                }
            }

            // Single trailing number: `file:line`
            if !prefix.is_empty() {
                return (prefix.to_string(), Some(last_num.max(1)), None);
            }
        }
    }

    (raw.to_string(), None, None)
}

/// Scans raw command-line arguments and extracts any `+line[:col]` jump option,
/// returning the filtered arguments for clap together with extracted line/column.
pub fn extract_plus_jump(raw_args: &[OsString]) -> (Vec<OsString>, Option<usize>, Option<usize>) {
    let mut filtered = Vec::with_capacity(raw_args.len());
    let mut jump_line = None;
    let mut jump_col = None;
    let mut seen_dash_dash = false;

    for (i, arg) in raw_args.iter().enumerate() {
        if i == 0 {
            filtered.push(arg.clone());
            continue;
        }

        if let Some(s) = arg.to_str() {
            if s == "--" {
                seen_dash_dash = true;
                filtered.push(arg.clone());
                continue;
            }

            if !seen_dash_dash && s.starts_with('+') {
                if let Some((line, col)) = parse_line_col_str(&s[1..]) {
                    if jump_line.is_none() {
                        jump_line = Some(line);
                        jump_col = col;
                    }
                    continue;
                }
            }
        }

        filtered.push(arg.clone());
    }

    (filtered, jump_line, jump_col)
}

pub fn parse_options_from(args: &[OsString]) -> Result<Options, clap::Error> {
    let (filtered_args, plus_line, plus_col) = extract_plus_jump(args);
    let matches = build_command().try_get_matches_from(filtered_args)?;

    let is_full = matches.get_flag("full");
    let lines = matches.get_one::<u16>("lines").unwrap();
    let raw_file = matches.get_one::<String>("file").unwrap();
    let format = matches.get_one::<String>("format").cloned();
    let backup = matches.get_flag("backup");

    let (file, file_line, file_col) = parse_file_and_location(raw_file);
    let (line, column) = match (plus_line, plus_col) {
        (Some(l), c) => (Some(l), c),
        (None, _) => (file_line, file_col),
    };

    let mode = match is_full {
        true => TUImode::FULL,
        false => TUImode::INLINE(*lines),
    };

    Ok(Options {
        mode,
        file,
        format,
        line,
        column,
        backup,
    })
}

pub fn get_options() -> Options {
    let args: Vec<OsString> = std::env::args_os().collect();
    parse_options_from(&args).unwrap_or_else(|e| e.exit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_col_str() {
        assert_eq!(parse_line_col_str("42"), Some((42, None)));
        assert_eq!(parse_line_col_str("42:15"), Some((42, Some(15))));
        assert_eq!(parse_line_col_str("0"), Some((1, None)));
        assert_eq!(parse_line_col_str("0:0"), Some((1, Some(1))));
        assert_eq!(parse_line_col_str("abc"), None);
        assert_eq!(parse_line_col_str(""), None);
        assert_eq!(parse_line_col_str("42:abc"), None);
    }

    #[test]
    fn test_parse_file_and_location() {
        assert_eq!(
            parse_file_and_location("config.toml:45"),
            ("config.toml".to_string(), Some(45), None)
        );
        assert_eq!(
            parse_file_and_location("src/main.rs:82:15"),
            ("src/main.rs".to_string(), Some(82), Some(15))
        );
        assert_eq!(
            parse_file_and_location("src/main.rs"),
            ("src/main.rs".to_string(), None, None)
        );
        assert_eq!(
            parse_file_and_location("path/with:colon/file.rs:10"),
            ("path/with:colon/file.rs".to_string(), Some(10), None)
        );
        // Literal existing file (Cargo.toml exists in cwd)
        assert_eq!(
            parse_file_and_location("Cargo.toml"),
            ("Cargo.toml".to_string(), None, None)
        );
    }

    #[test]
    fn test_parse_options_from_file_colon_line() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("config.toml:45"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "config.toml");
        assert_eq!(opts.line, Some(45));
        assert_eq!(opts.column, None);
        assert_eq!(opts.mode, TUImode::INLINE(8));
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_from_file_colon_line_col() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("main.rs:82:15"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "main.rs");
        assert_eq!(opts.line, Some(82));
        assert_eq!(opts.column, Some(15));
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_from_plus_line_file() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("+45"),
            OsString::from("config.toml"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "config.toml");
        assert_eq!(opts.line, Some(45));
        assert_eq!(opts.column, None);
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_from_file_plus_line() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("config.toml"),
            OsString::from("+45:20"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "config.toml");
        assert_eq!(opts.line, Some(45));
        assert_eq!(opts.column, Some(20));
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_with_flags() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("-l"),
            OsString::from("16"),
            OsString::from("+100"),
            OsString::from("large_file.txt"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "large_file.txt");
        assert_eq!(opts.line, Some(100));
        assert_eq!(opts.mode, TUImode::INLINE(16));
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_fullscreen() {
        let args = vec![
            OsString::from("ie"),
            OsString::from("--full"),
            OsString::from("src/lib.rs:50:3"),
        ];
        let opts = parse_options_from(&args).unwrap();
        assert_eq!(opts.file, "src/lib.rs");
        assert_eq!(opts.line, Some(50));
        assert_eq!(opts.column, Some(3));
        assert_eq!(opts.mode, TUImode::FULL);
        assert!(!opts.backup);
    }

    #[test]
    fn test_parse_options_backup_flags() {
        let args_long = vec![
            OsString::from("ie"),
            OsString::from("--backup"),
            OsString::from("config.toml"),
        ];
        let opts_long = parse_options_from(&args_long).unwrap();
        assert!(opts_long.backup);

        let args_short = vec![
            OsString::from("ie"),
            OsString::from("-b"),
            OsString::from("config.toml"),
        ];
        let opts_short = parse_options_from(&args_short).unwrap();
        assert!(opts_short.backup);
    }
}
