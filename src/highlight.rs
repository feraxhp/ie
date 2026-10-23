use std::fs;
use std::path::Path;
use texera::{include_languaje_dir, HighlightPlugin, LanguageSpec, LanguageToml, ThemeMap};
#[cfg(test)]
use texera::{LanguageRegistry, LexState};

/// Detects the language or syntax format from the given file name or extension.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    // 1. Exact file name and prefix detection
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = file_name.to_ascii_lowercase();
        match lower.as_str() {
            ".gitignore" | ".gitattributes" | ".gitmodules" | "exclude" | ".gitkeep" => {
                return Some("gitignore");
            }
            "commit_editmsg" | "merge_msg" | "tag_editmsg" | "edit_description" => {
                return Some("gitcommit");
            }
            "git-rebase-todo" | "git-rebase-todo.backup" | "rebase-todo" => {
                return Some("gitrebase");
            }
            ".gitconfig" | "gitconfig" => {
                return Some("gitconfig");
            }
            "cargo.lock" => return Some("toml"),
            "dockerfile" | "containerfile" => return Some("dockerfile"),
            "makefile" | "gnumakefile" | "justfile" => return Some("makefile"),
            ".env" | "environment" => return Some("env"),
            "hosts" | "resolv.conf" => return Some("hosts"),
            "fstab" | "crypttab" | "mtab" => return Some("fstab"),
            "sudoers" => return Some("sudoers"),
            "crontab" => return Some("crontab"),
            "override.conf" => return Some("systemd"),
            "ssh_config" | "sshd_config" | "known_hosts" | "authorized_keys" => {
                return Some("sshconfig");
            }
            "nginx.conf" => return Some("nginx"),
            "caddyfile" => return Some("caddy"),
            "sources.list" => return Some("apt_sources"),

            // Direct language identifiers & codeblock tags
            "rust" | "rs" => return Some("rust"),
            "python" | "python3" | "py" | "pyw" | "pyi" => return Some("python"),
            "javascript" | "js" | "mjs" | "cjs" | "jsx" => return Some("javascript"),
            "typescript" | "ts" | "mts" | "cts" | "tsx" => return Some("typescript"),
            "c" | "h" => return Some("c"),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "c++" => return Some("cpp"),
            "c_sharp" | "csharp" | "c#" | "cs" => return Some("c_sharp"),
            "go" | "golang" => return Some("go"),
            "kotlin" | "kt" | "kts" => return Some("kotlin"),
            "java" => return Some("java"),
            "html" | "htm" => return Some("html"),
            "css" | "scss" | "sass" | "less" => return Some("css"),
            "json" => return Some("json"),
            "toml" => return Some("toml"),
            "yaml" | "yml" => return Some("yaml"),
            "shell" | "bash" | "zsh" | "fish" | "sh" => return Some("shell"),
            "markdown" | "md" => return Some("markdown"),
            "lua" => return Some("lua"),
            "sql" => return Some("sql"),
            "diff" | "patch" => return Some("diff"),
            "docker" => return Some("dockerfile"),
            "make" | "mk" | "mak" => return Some("makefile"),
            "rebase" => return Some("gitrebase"),
            "commit" => return Some("gitcommit"),
            "env" | "dotenv" => return Some("env"),
            "desktop" => return Some("desktop"),
            "systemd" => return Some("systemd"),
            "ini" | "conf" => return Some("ini"),
            "nginx" => return Some("nginx"),
            "caddy" => return Some("caddy"),
            "apt" | "apt_sources" | "sources" | "repo" => return Some("apt_sources"),
            "cron" => return Some("crontab"),
            _ => {}
        }
        if lower.starts_with("git-rebase") {
            return Some("gitrebase");
        }
        if lower.starts_with("commit_editmsg")
            || lower.starts_with("merge_msg")
            || lower.starts_with("tag_editmsg")
        {
            return Some("gitcommit");
        }
        if lower.starts_with(".git") {
            return Some("gitconfig");
        }
        if lower.starts_with(".env.") || lower.ends_with(".env") {
            return Some("env");
        }
        if lower.starts_with("sudoers") {
            return Some("sudoers");
        }
        if lower.starts_with("crontab") {
            return Some("crontab");
        }
        if lower == "config" {
            if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
                if parent.eq_ignore_ascii_case(".ssh") || parent.eq_ignore_ascii_case("ssh") {
                    return Some("sshconfig");
                }
            }
        }
    }

    // Contextual parent directory detection (e.g. /etc/sudoers.d/admin, /etc/cron.d/anacron, systemd unit drop-ins)
    if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
        let parent_lower = parent.to_ascii_lowercase();
        if parent_lower == "sudoers.d" {
            return Some("sudoers");
        }
        if parent_lower == "cron.d"
            || parent_lower == "cron.daily"
            || parent_lower == "cron.hourly"
            || parent_lower == "cron.weekly"
            || parent_lower == "cron.monthly"
        {
            return Some("crontab");
        }
        if parent_lower.ends_with(".service.d")
            || parent_lower.ends_with(".socket.d")
            || parent_lower.ends_with(".target.d")
            || parent_lower.ends_with(".timer.d")
            || parent_lower.ends_with(".mount.d")
            || parent_lower.ends_with(".automount.d")
            || parent_lower.ends_with(".path.d")
            || parent_lower.ends_with(".slice.d")
            || parent_lower.ends_with(".scope.d")
            || parent_lower.ends_with(".conf.d")
        {
            return Some("systemd");
        }
    }

    // Systemd drop-in snippets or system configuration files (e.g. /etc/systemd/system.conf, /etc/systemd/system/.../override.conf)
    if path.components().any(|c| c.as_os_str().to_str().map(|s| s.eq_ignore_ascii_case("systemd")).unwrap_or(false)) {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("conf") {
                return Some("systemd");
            }
        }
    }

    // 2. Extension detection
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        return match ext_lower.as_str() {
            "rs" | "rust" => Some("rust"),
            "kt" | "kts" | "kotlin" => Some("kotlin"),
            "js" | "mjs" | "cjs" | "jsx" | "javascript" => Some("javascript"),
            "ts" | "mts" | "cts" | "tsx" | "typescript" => Some("typescript"),
            "py" | "pyw" | "pyi" | "python" | "python3" => Some("python"),
            "go" | "golang" => Some("go"),
            "c" | "h" => Some("c"),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "c++" => Some("cpp"),
            "cs" | "csharp" | "c_sharp" | "c#" => Some("c_sharp"),
            "java" => Some("java"),
            "html" | "htm" => Some("html"),
            "css" | "scss" | "sass" | "less" => Some("css"),
            "json" => Some("json"),
            "toml" => Some("toml"),
            "yaml" | "yml" => Some("yaml"),
            "sh" | "bash" | "zsh" | "fish" | "shell" => Some("shell"),
            "md" | "markdown" => Some("markdown"),
            "lua" => Some("lua"),
            "sql" => Some("sql"),
            "diff" | "patch" => Some("diff"),
            "rebase" | "gitrebase" => Some("gitrebase"),
            "commit" | "gitcommit" => Some("gitcommit"),
            "mk" | "mak" | "make" | "makefile" => Some("makefile"),
            "dockerfile" | "docker" => Some("dockerfile"),
            "env" | "dotenv" => Some("env"),
            "sshconfig" | "ssh_config" | "ssh" => Some("sshconfig"),
            "sudoers" => Some("sudoers"),
            "cron" | "crontab" => Some("crontab"),
            // Linux system files
            "desktop" => Some("desktop"),
            "service" | "socket" | "target" | "timer" | "mount" | "automount" | "path"
            | "slice" | "scope" | "systemd" => Some("systemd"),
            "ini" | "conf" => Some("ini"),
            "nginx" => Some("nginx"),
            "caddy" | "caddyfile" => Some("caddy"),
            "sources" | "repo" | "apt_sources" | "apt" => Some("apt_sources"),
            _ => None,
        };
    }

    None
}

/// Resolves a codeblock language identifier or alias to a canonical language name.
/// Reuses `detect_language` both as a direct file/path name and as a file extension.
pub fn resolve_codeblock_language(tag: &str) -> Option<&'static str> {
    let clean = tag.trim();
    if clean.is_empty() {
        return None;
    }
    // 1. Direct path/file name match
    if let Some(lang) = detect_language(Path::new(clean)) {
        return Some(lang);
    }
    // 2. Extension match (e.g. "file.<tag>")
    let dummy = format!("file.{}", clean);
    if let Some(lang) = detect_language(Path::new(&dummy)) {
        return Some(lang);
    }
    None
}

/// Returns the built-in TOML grammar source for the given canonical language name.
pub fn get_builtin_toml(lang: &str) -> Option<&'static str> {
    match lang {
        "apt_sources" => Some(include_str!("../languages/apt_sources/apt_sources.toml")),
        "c" => Some(include_str!("../languages/c/c.toml")),
        "c_sharp" => Some(include_str!("../languages/c_sharp/c_sharp.toml")),
        "caddy" => Some(include_str!("../languages/caddy/caddy.toml")),
        "cpp" => Some(include_str!("../languages/cpp/cpp.toml")),
        "crontab" | "cron" => Some(include_str!("../languages/crontab/crontab.toml")),
        "css" => Some(include_str!("../languages/css/css.toml")),
        "desktop" => Some(include_str!("../languages/desktop/desktop.toml")),
        "diff" => Some(include_str!("../languages/diff/diff.toml")),
        "dockerfile" => Some(include_str!("../languages/dockerfile/dockerfile.toml")),
        "env" => Some(include_str!("../languages/env/env.toml")),
        "fstab" => Some(include_str!("../languages/fstab/fstab.toml")),
        "gitcommit" => Some(include_str!("../languages/gitcommit/gitcommit.toml")),
        "gitconfig" => Some(include_str!("../languages/gitconfig/gitconfig.toml")),
        "gitignore" => Some(include_str!("../languages/gitignore/gitignore.toml")),
        "gitrebase" => Some(include_str!("../languages/gitrebase/gitrebase.toml")),
        "go" => Some(include_str!("../languages/go/go.toml")),
        "hosts" => Some(include_str!("../languages/hosts/hosts.toml")),
        "html" => Some(include_str!("../languages/html/html.toml")),
        "ini" => Some(include_str!("../languages/ini/ini.toml")),
        "java" => Some(include_str!("../languages/java/java.toml")),
        "javascript" => Some(include_str!("../languages/javascript/javascript.toml")),
        "json" => Some(include_str!("../languages/json/json.toml")),
        "kotlin" => Some(include_str!("../languages/kotlin/kotlin.toml")),
        "lua" => Some(include_str!("../languages/lua/lua.toml")),
        "makefile" => Some(include_str!("../languages/makefile/makefile.toml")),
        "markdown" => Some(include_str!("../languages/markdown/markdown.toml")),
        "nginx" => Some(include_str!("../languages/nginx/nginx.toml")),
        "python" => Some(include_str!("../languages/python/python.toml")),
        "rust" => Some(include_str!("../languages/rust/rust.toml")),
        "shell" => Some(include_str!("../languages/shell/shell.toml")),
        "sql" => Some(include_str!("../languages/sql/sql.toml")),
        "sshconfig" => Some(include_str!("../languages/sshconfig/sshconfig.toml")),
        "sudoers" => Some(include_str!("../languages/sudoers/sudoers.toml")),
        "systemd" => Some(include_str!("../languages/systemd/systemd.toml")),
        "toml" => Some(include_str!("../languages/toml/toml.toml")),
        "typescript" => Some(include_str!("../languages/typescript/typescript.toml")),
        "yaml" => Some(include_str!("../languages/yaml/yaml.toml")),
        _ => None,
    }
}

/// Returns the built-in theme for the given canonical language name, if available.
pub fn get_builtin_theme(lang: &str) -> Option<ThemeMap> {
    let json_str = match lang {
        "apt_sources" => Some(include_str!("../languages/apt_sources/theme.json")),
        "c" => Some(include_str!("../languages/c/theme.json")),
        "c_sharp" => Some(include_str!("../languages/c_sharp/theme.json")),
        "caddy" => Some(include_str!("../languages/caddy/theme.json")),
        "cpp" => Some(include_str!("../languages/cpp/theme.json")),
        "crontab" | "cron" => Some(include_str!("../languages/crontab/theme.json")),
        "css" => Some(include_str!("../languages/css/theme.json")),
        "desktop" => Some(include_str!("../languages/desktop/theme.json")),
        "diff" => Some(include_str!("../languages/diff/theme.json")),
        "dockerfile" => Some(include_str!("../languages/dockerfile/theme.json")),
        "env" => Some(include_str!("../languages/env/theme.json")),
        "fstab" => Some(include_str!("../languages/fstab/theme.json")),
        "gitcommit" => Some(include_str!("../languages/gitcommit/theme.json")),
        "gitconfig" => Some(include_str!("../languages/gitconfig/theme.json")),
        "gitignore" => Some(include_str!("../languages/gitignore/theme.json")),
        "gitrebase" => Some(include_str!("../languages/gitrebase/theme.json")),
        "go" => Some(include_str!("../languages/go/theme.json")),
        "hosts" => Some(include_str!("../languages/hosts/theme.json")),
        "html" => Some(include_str!("../languages/html/theme.json")),
        "ini" => Some(include_str!("../languages/ini/theme.json")),
        "java" => Some(include_str!("../languages/java/theme.json")),
        "javascript" => Some(include_str!("../languages/javascript/theme.json")),
        "json" => Some(include_str!("../languages/json/theme.json")),
        "kotlin" => Some(include_str!("../languages/kotlin/theme.json")),
        "lua" => Some(include_str!("../languages/lua/theme.json")),
        "makefile" => Some(include_str!("../languages/makefile/theme.json")),
        "markdown" => Some(include_str!("../languages/markdown/theme.json")),
        "nginx" => Some(include_str!("../languages/nginx/theme.json")),
        "python" => Some(include_str!("../languages/python/theme.json")),
        "rust" => Some(include_str!("../languages/rust/theme.json")),
        "shell" => Some(include_str!("../languages/shell/theme.json")),
        "sql" => Some(include_str!("../languages/sql/theme.json")),
        "sshconfig" => Some(include_str!("../languages/sshconfig/theme.json")),
        "sudoers" => Some(include_str!("../languages/sudoers/theme.json")),
        "systemd" => Some(include_str!("../languages/systemd/theme.json")),
        "toml" => Some(include_str!("../languages/toml/theme.json")),
        "typescript" => Some(include_str!("../languages/typescript/theme.json")),
        "yaml" => Some(include_str!("../languages/yaml/theme.json")),
        _ => None,
    };
    json_str.and_then(|s| ThemeMap::from_json_str(s).ok())
}

/// Loads a language's specification (grammar and theme) on demand for codeblock syntax highlighting.
/// Reuses `detect_language` to resolve aliases or extensions to canonical language specifications.
/// Checks user configuration directory first, then falls back to built-in grammars and themes.
pub fn load_codeblock_spec(tag: &str, config_dir: &Path) -> Option<LanguageSpec> {
    let clean = tag.trim();
    if clean.is_empty() {
        return None;
    }

    // Resolve canonical language identifier using detect_language
    let lang = resolve_codeblock_language(clean).unwrap_or(clean);

    // 1. Check user config overrides first
    if config_dir.exists() && config_dir.is_dir() {
        let grammar_candidates = [
            config_dir.join(lang).join(format!("{}.toml", lang)),
            config_dir.join(lang).join("syntax.toml"),
            config_dir.join(format!("{}.toml", lang)),
            config_dir.join(clean).join(format!("{}.toml", clean)),
            config_dir.join(format!("{}.toml", clean)),
        ];

        for candidate in &grammar_candidates {
            if candidate.is_file() {
                if let Ok(grammar) = fs::read_to_string(candidate) {
                    let theme_candidates = [
                        candidate.with_extension("json"),
                        candidate.parent().map(|p| p.join("theme.json")).unwrap_or_default(),
                        config_dir.join(lang).join("theme.json"),
                        config_dir.join(format!("{}.json", lang)),
                    ];
                    let mut user_theme = None;
                    for tc in &theme_candidates {
                        if tc.is_file() {
                            if let Ok(theme) = ThemeMap::load_from_file(tc) {
                                user_theme = Some(theme);
                                break;
                            }
                        }
                    }
                    let theme = user_theme.or_else(|| get_builtin_theme(lang));
                    return Some(LanguageSpec::new(grammar, theme));
                }
            }
        }
    }

    // 2. Fall back to built-in language definitions
    let grammar = get_builtin_toml(lang)
        .or_else(|| get_builtin_toml(clean))?
        .to_string();

    let theme = get_builtin_theme(lang).or_else(|| get_builtin_theme(clean));

    Some(LanguageSpec::new(grammar, theme))
}

/// Convenience function that loads only the TOML grammar definition.
#[allow(dead_code)]
pub fn load_codeblock_toml(tag: &str, config_dir: &Path) -> Option<String> {
    load_codeblock_spec(tag, config_dir).map(|spec| spec.grammar)
}

/// Populates global fallback theme with standard tokens so embedded codeblocks have colors.
fn init_global_theme(theme: &mut ThemeMap) {
    let defaults = [
        ("keyword", ratatui::style::Color::Magenta),
        ("function", ratatui::style::Color::Blue),
        ("type", ratatui::style::Color::Yellow),
        ("string", ratatui::style::Color::Green),
        ("number", ratatui::style::Color::Yellow),
        ("boolean", ratatui::style::Color::Yellow),
        ("constant", ratatui::style::Color::Yellow),
        ("variable", ratatui::style::Color::LightBlue),
        ("operator", ratatui::style::Color::Cyan),
        ("comment", ratatui::style::Color::DarkGray),
        ("punctuation.bracket", ratatui::style::Color::DarkGray),
        ("punctuation.delimiter", ratatui::style::Color::DarkGray),
        ("punctuation.special", ratatui::style::Color::Magenta),
        ("attribute", ratatui::style::Color::Magenta),
        ("markup.heading", ratatui::style::Color::Yellow),
        ("markup.raw", ratatui::style::Color::Green),
        ("markup.bold", ratatui::style::Color::Yellow),
        ("markup.italic", ratatui::style::Color::Magenta),
    ];
    for (token, color) in defaults {
        if !theme.token_colors.contains_key(token) {
            theme.set_color(token, color);
        }
    }
}

/// Applies user overrides from `config_dir` onto the given highlighter.
/// Files and directories in `config_dir` take strict precedence over built-in specifications:
/// - `global_theme.json` or `theme.json` in the root overrides global fallback token colors.
/// - Subdirectories `config_dir/<lang>/` with `*.toml` and/or `theme.json` override grammar and token colors.
/// - Flat files `config_dir/<lang>.toml` and `config_dir/<lang>.json` override grammar and token colors.
fn apply_config_overrides(highlight: &mut HighlightPlugin, config_dir: &Path) {
    if !config_dir.exists() || !config_dir.is_dir() {
        return;
    }

    // 1. Root theme overrides for global fallback colors
    let root_global_theme = config_dir.join("global_theme.json");
    let root_theme = config_dir.join("theme.json");
    if root_global_theme.is_file() {
        if let Ok(theme) = ThemeMap::load_from_file(&root_global_theme) {
            for (token, color) in theme.token_colors {
                highlight.global_theme.set_color(token, color);
            }
        }
    } else if root_theme.is_file() {
        if let Ok(theme) = ThemeMap::load_from_file(&root_theme) {
            for (token, color) in theme.token_colors {
                highlight.global_theme.set_color(token, color);
            }
        }
    }

    // 2. Scan entries in config_dir for language directories and flat override files
    if let Ok(entries) = fs::read_dir(config_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Subdirectory mode: config_dir/<lang>/...
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    let lang_name = dir_name;
                    let toml_candidates = [
                        path.join(format!("{}.toml", lang_name)),
                        path.join("syntax.toml"),
                    ];
                    let toml_content = toml_candidates
                        .iter()
                        .find(|p| p.is_file())
                        .and_then(|p| fs::read_to_string(p).ok());

                    let theme_path = path.join("theme.json");
                    let user_theme = if theme_path.is_file() {
                        ThemeMap::load_from_file(&theme_path).ok()
                    } else {
                        None
                    };

                    if toml_content.is_some() || user_theme.is_some() {
                        override_or_insert_language(
                            highlight,
                            lang_name,
                            toml_content,
                            user_theme,
                        );
                    }
                }
            } else if path.is_file() {
                // Flat file mode: config_dir/<lang>.toml and config_dir/<lang>.json
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                if ext == "toml" {
                    if let Some(lang_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let json_path = path.with_extension("json");
                            let user_theme = if json_path.is_file() {
                                ThemeMap::load_from_file(&json_path).ok()
                            } else {
                                None
                            };
                            override_or_insert_language(
                                highlight,
                                lang_name,
                                Some(content),
                                user_theme,
                            );
                        }
                    }
                } else if ext == "json" {
                    // Language-specific theme override without a companion .toml file
                    if let Some(lang_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if lang_name != "theme" && lang_name != "global_theme" {
                            let companion_toml = path.with_extension("toml");
                            if !companion_toml.exists() {
                                if let Ok(theme) = ThemeMap::load_from_file(&path) {
                                    override_or_insert_language(
                                        highlight,
                                        lang_name,
                                        None,
                                        Some(theme),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn override_or_insert_language(
    highlight: &mut HighlightPlugin,
    lang_name: &str,
    toml_content: Option<String>,
    user_theme: Option<ThemeMap>,
) {
    if let Some(ref content) = toml_content {
        let _ = highlight.register_language(lang_name, content);
    }

    if let Some(existing) = highlight.languages.get_mut(lang_name) {
        // Language exists in built-ins: user configuration overrides built-in values
        if let Some(theme) = user_theme {
            for (token, color) in theme.token_colors {
                existing.theme.set_color(token, color);
            }
        }
        if let Some(content) = toml_content {
            existing.source = content;
        }
    } else if let Some(content) = toml_content {
        // New language definition from user config
        let theme = user_theme.unwrap_or_default();
        highlight.languages.insert(
            lang_name.to_string(),
            LanguageToml::new(lang_name, content, theme),
        );
    }
}

/// Initializes the syntax highlighter with built-in languages loaded via `include_languaje_dir!`
/// and extends them at runtime with dynamic user queries and themes from the configuration directory.
/// Installs an on-demand language specification loader callback (grammar + theme) for Markdown codeblocks.
pub fn init_highlighter(config_dir: &Path) -> HighlightPlugin {
    // 1. Load built-in project languages using the Texera proc-macro
    let mut highlight = include_languaje_dir!("languages");

    // Initialize global fallback theme for embedded languages
    init_global_theme(&mut highlight.global_theme);

    // 2. Load dynamic specs with strict precedence: config_dir overrides built-ins
    apply_config_overrides(&mut highlight, config_dir);

    // 3. Register on-demand spec loader for codeblocks, resolving grammar and contextual theme
    let config_dir_buf = config_dir.to_path_buf();
    highlight = highlight.with_spec_loader(move |name: &str| {
        load_codeblock_spec(name, &config_dir_buf)
    });

    highlight
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use std::path::PathBuf;

    #[test]
    fn test_detect_common_languages() {
        assert_eq!(detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("Main.kt")), Some("kotlin"));
        assert_eq!(detect_language(Path::new("build.gradle.kts")), Some("kotlin"));
        assert_eq!(detect_language(Path::new("app.js")), Some("javascript"));
        assert_eq!(detect_language(Path::new("app.tsx")), Some("typescript"));
        assert_eq!(detect_language(Path::new("script.py")), Some("python"));
        assert_eq!(detect_language(Path::new("server.go")), Some("go"));
        assert_eq!(detect_language(Path::new("main.c")), Some("c"));
        assert_eq!(detect_language(Path::new("main.cpp")), Some("cpp"));
        assert_eq!(detect_language(Path::new("Program.cs")), Some("c_sharp"));
        assert_eq!(detect_language(Path::new("App.java")), Some("java"));
        assert_eq!(detect_language(Path::new("index.html")), Some("html"));
        assert_eq!(detect_language(Path::new("style.css")), Some("css"));
        assert_eq!(detect_language(Path::new("data.json")), Some("json"));
        assert_eq!(detect_language(Path::new("Cargo.toml")), Some("toml"));
        assert_eq!(detect_language(Path::new("config.yaml")), Some("yaml"));
        assert_eq!(detect_language(Path::new("deploy.sh")), Some("shell"));
        assert_eq!(detect_language(Path::new("README.md")), Some("markdown"));
        assert_eq!(detect_language(Path::new("init.lua")), Some("lua"));
        assert_eq!(detect_language(Path::new("schema.sql")), Some("sql"));
        assert_eq!(detect_language(Path::new("changes.diff")), Some("diff"));
        assert_eq!(detect_language(Path::new("feature.patch")), Some("diff"));
        assert_eq!(detect_language(Path::new("Makefile")), Some("makefile"));
        assert_eq!(detect_language(Path::new("rules.mk")), Some("makefile"));
        assert_eq!(detect_language(Path::new("Dockerfile")), Some("dockerfile"));
        assert_eq!(detect_language(Path::new("app.dockerfile")), Some("dockerfile"));
    }

    #[test]
    fn test_detect_linux_config_files() {
        assert_eq!(detect_language(Path::new("app.desktop")), Some("desktop"));
        assert_eq!(detect_language(Path::new("docker.service")), Some("systemd"));
        assert_eq!(detect_language(Path::new("docker.socket")), Some("systemd"));
        assert_eq!(detect_language(Path::new("backup.timer")), Some("systemd"));
        assert_eq!(detect_language(Path::new("settings.conf")), Some("ini"));
        assert_eq!(detect_language(Path::new("config.ini")), Some("ini"));
        // Linux system files
        assert_eq!(detect_language(Path::new(".env")), Some("env"));
        assert_eq!(detect_language(Path::new(".env.local")), Some("env"));
        assert_eq!(detect_language(Path::new("/etc/hosts")), Some("hosts"));
        assert_eq!(detect_language(Path::new("/etc/resolv.conf")), Some("hosts"));
        assert_eq!(detect_language(Path::new("/etc/fstab")), Some("fstab"));
        assert_eq!(detect_language(Path::new("/etc/crypttab")), Some("fstab"));
        assert_eq!(detect_language(Path::new("/etc/sudoers")), Some("sudoers"));
        assert_eq!(detect_language(Path::new("ssh_config")), Some("sshconfig"));
        assert_eq!(detect_language(Path::new("samples/ssh_config")), Some("sshconfig"));
        assert_eq!(detect_language(Path::new("/etc/ssh/ssh_config")), Some("sshconfig"));
        assert_eq!(detect_language(Path::new("/etc/ssh/sshd_config")), Some("sshconfig"));
        assert_eq!(detect_language(Path::new("/home/user/.ssh/config")), Some("sshconfig"));
        assert_eq!(detect_language(Path::new("/etc/nginx/nginx.conf")), Some("nginx"));
        assert_eq!(detect_language(Path::new("Caddyfile")), Some("caddy"));
        assert_eq!(detect_language(Path::new("/etc/apt/sources.list")), Some("apt_sources"));
        assert_eq!(detect_language(Path::new("fedora.repo")), Some("apt_sources"));

        // Crontab detection
        assert_eq!(detect_language(Path::new("crontab")), Some("crontab"));
        assert_eq!(detect_language(Path::new("/tmp/crontab.123456")), Some("crontab"));
        assert_eq!(detect_language(Path::new("/tmp/crontab.uY6aBc/crontab")), Some("crontab"));
        assert_eq!(detect_language(Path::new("/etc/crontab")), Some("crontab"));
        assert_eq!(detect_language(Path::new("/etc/cron.d/anacron")), Some("crontab"));
        assert_eq!(detect_language(Path::new("backup.cron")), Some("crontab"));
        assert_eq!(detect_language(Path::new("samples/crontab")), Some("crontab"));

        // Sudoers detection
        assert_eq!(detect_language(Path::new("/etc/sudoers.tmp")), Some("sudoers"));
        assert_eq!(detect_language(Path::new("samples/sudoers.tmp")), Some("sudoers"));
        assert_eq!(detect_language(Path::new("/etc/sudoers.d/01-wheel")), Some("sudoers"));
        assert_eq!(detect_language(Path::new("/etc/sudoers.d/admin")), Some("sudoers"));

        // Systemd drop-in and config files
        assert_eq!(detect_language(Path::new("override.conf")), Some("systemd"));
        assert_eq!(detect_language(Path::new("samples/override.conf")), Some("systemd"));
        assert_eq!(
            detect_language(Path::new("/etc/systemd/system/docker.service.d/override.conf")),
            Some("systemd")
        );
        assert_eq!(
            detect_language(Path::new("/etc/systemd/system/nginx.service.d/50-limits.conf")),
            Some("systemd")
        );
        assert_eq!(detect_language(Path::new("/etc/systemd/system.conf")), Some("systemd"));
        assert_eq!(detect_language(Path::new("/etc/systemd/user.conf")), Some("systemd"));
    }

    #[test]
    fn test_detect_git_files() {
        assert_eq!(detect_language(Path::new(".gitignore")), Some("gitignore"));
        assert_eq!(detect_language(Path::new(".gitattributes")), Some("gitignore"));
        assert_eq!(detect_language(Path::new(".gitmodules")), Some("gitignore"));
        assert_eq!(detect_language(Path::new("COMMIT_EDITMSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new("samples/COMMIT_EDITMSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new("MERGE_MSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new("TAG_EDITMSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new(".git/COMMIT_EDITMSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new(".gitconfig")), Some("gitconfig"));
        assert_eq!(detect_language(Path::new("gitconfig")), Some("gitconfig"));
        assert_eq!(detect_language(Path::new("git-rebase-todo")), Some("gitrebase"));
        assert_eq!(detect_language(Path::new(".git/rebase-merge/git-rebase-todo")), Some("gitrebase"));
        assert_eq!(detect_language(Path::new("todo.rebase")), Some("gitrebase"));
        assert_eq!(detect_language(Path::new("samples/git-rebase-todo")), Some("gitrebase"));
    }

    #[test]
    fn test_resolve_codeblock_language() {
        assert_eq!(resolve_codeblock_language("rust"), Some("rust"));
        assert_eq!(resolve_codeblock_language("rs"), Some("rust"));
        assert_eq!(resolve_codeblock_language("python"), Some("python"));
        assert_eq!(resolve_codeblock_language("py"), Some("python"));
        assert_eq!(resolve_codeblock_language("javascript"), Some("javascript"));
        assert_eq!(resolve_codeblock_language("js"), Some("javascript"));
        assert_eq!(resolve_codeblock_language("typescript"), Some("typescript"));
        assert_eq!(resolve_codeblock_language("ts"), Some("typescript"));
        assert_eq!(resolve_codeblock_language("c++"), Some("cpp"));
        assert_eq!(resolve_codeblock_language("cpp"), Some("cpp"));
        assert_eq!(resolve_codeblock_language("c#"), Some("c_sharp"));
        assert_eq!(resolve_codeblock_language("cs"), Some("c_sharp"));
        assert_eq!(resolve_codeblock_language("bash"), Some("shell"));
        assert_eq!(resolve_codeblock_language("sh"), Some("shell"));
        assert_eq!(resolve_codeblock_language("docker"), Some("dockerfile"));
        assert_eq!(resolve_codeblock_language("make"), Some("makefile"));
        assert_eq!(resolve_codeblock_language("rebase"), Some("gitrebase"));
        assert_eq!(resolve_codeblock_language("crontab"), Some("crontab"));
        assert_eq!(resolve_codeblock_language("cron"), Some("crontab"));
        assert_eq!(resolve_codeblock_language("unknown_lang_123"), None);
    }

    #[test]
    fn test_load_codeblock_spec() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let rust_spec = load_codeblock_spec("rust", &dummy_dir).expect("rust spec must load");
        assert!(rust_spec.grammar.contains("name = \"rust\""));
        assert!(rust_spec.theme.is_some());

        let rs_spec = load_codeblock_spec("rs", &dummy_dir).expect("rs spec must load");
        assert!(rs_spec.grammar.contains("name = \"rust\""));
        assert!(rs_spec.theme.is_some());

        let py_spec = load_codeblock_spec("py", &dummy_dir).expect("py spec must load");
        assert!(py_spec.grammar.contains("name = \"python\""));
        assert!(py_spec.theme.is_some());

        let cron_spec = load_codeblock_spec("crontab", &dummy_dir).expect("crontab spec must load");
        assert!(cron_spec.grammar.contains("name = \"crontab\""));
        assert!(cron_spec.theme.is_some());

        assert!(load_codeblock_spec("unknown_tag_xyz", &dummy_dir).is_none());
    }

    #[test]
    fn test_load_codeblock_toml() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        assert!(load_codeblock_toml("rust", &dummy_dir).is_some());
        assert!(load_codeblock_toml("rs", &dummy_dir).is_some());
        assert!(load_codeblock_toml("python", &dummy_dir).is_some());
        assert!(load_codeblock_toml("py", &dummy_dir).is_some());
        assert!(load_codeblock_toml("c++", &dummy_dir).is_some());
        assert!(load_codeblock_toml("c#", &dummy_dir).is_some());
        assert!(load_codeblock_toml("bash", &dummy_dir).is_some());
        assert!(load_codeblock_toml("crontab", &dummy_dir).is_some());
        assert!(load_codeblock_toml("unknown_tag_xyz", &dummy_dir).is_none());
    }

    #[test]
    fn test_builtin_scm_loaded_with_macro() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);

        assert!(
            highlighter.errors.is_empty(),
            "Built-in language compilation errors: {:?}",
            highlighter.errors
        );

        assert!(highlighter.languages.contains_key("rust"));
        assert!(highlighter.languages.contains_key("kotlin"));
        assert!(highlighter.languages.contains_key("javascript"));
        assert!(highlighter.languages.contains_key("typescript"));
        assert!(highlighter.languages.contains_key("python"));
        assert!(highlighter.languages.contains_key("go"));
        assert!(highlighter.languages.contains_key("c"));
        assert!(highlighter.languages.contains_key("cpp"));
        assert!(highlighter.languages.contains_key("c_sharp"));
        assert!(highlighter.languages.contains_key("java"));
        assert!(highlighter.languages.contains_key("html"));
        assert!(highlighter.languages.contains_key("css"));
        assert!(highlighter.languages.contains_key("json"));
        assert!(highlighter.languages.contains_key("toml"));
        assert!(highlighter.languages.contains_key("yaml"));
        assert!(highlighter.languages.contains_key("shell"));
        assert!(highlighter.languages.contains_key("markdown"));
        // Linux system files
        assert!(highlighter.languages.contains_key("systemd"));
        assert!(highlighter.languages.contains_key("desktop"));
        assert!(highlighter.languages.contains_key("ini"));
        // Git
        assert!(highlighter.languages.contains_key("gitignore"));
        assert!(highlighter.languages.contains_key("gitcommit"));
        assert!(highlighter.languages.contains_key("gitconfig"));
        assert!(highlighter.languages.contains_key("gitrebase"));
        // Additional DevOps and Linux formats
        assert!(highlighter.languages.contains_key("dockerfile"));
        assert!(highlighter.languages.contains_key("makefile"));
        assert!(highlighter.languages.contains_key("diff"));
        assert!(highlighter.languages.contains_key("env"));
        assert!(highlighter.languages.contains_key("hosts"));
        assert!(highlighter.languages.contains_key("fstab"));
        assert!(highlighter.languages.contains_key("sudoers"));
        assert!(highlighter.languages.contains_key("sshconfig"));
        assert!(highlighter.languages.contains_key("nginx"));
        assert!(highlighter.languages.contains_key("caddy"));
        assert!(highlighter.languages.contains_key("apt_sources"));
        assert!(highlighter.languages.contains_key("lua"));
        assert!(highlighter.languages.contains_key("sql"));
        assert!(highlighter.languages.contains_key("crontab"));
    }

    #[test]
    fn test_markdown_codeblocks_on_demand_loader() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);

        let md_lang_id = highlighter
            .registry()
            .lang_id("markdown")
            .expect("markdown must be registered");

        // 1. Line 0: Markdown heading
        let mut state = LexState::DOC_START;
        let t0 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "# Code Samples", &mut state);
        assert!(t0.iter().any(|t| t.2.as_ref() == "markup.heading"));

        // 2. Line 1: Opening code fence with alias "rs"
        let t1 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```rs", &mut state);
        assert!(t1.iter().any(|t| t.2.as_ref() == "punctuation.delimiter"));

        // 3. Line 2: Rust code inside the block
        let t2 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "fn main() {", &mut state);
        assert!(t2.iter().any(|t| t.2.as_ref().starts_with("keyword")));
        // Verify that the token is scoped to the embedded language, not markdown
        let rs_lang_id = highlighter.registry().lang_id("rs").expect("rs must be resolved");
        assert_eq!(t2[0].1, rs_lang_id);

        // Verify that contextual theme for "rs" is available
        let rs_theme = highlighter.theme_of_lang_id(rs_lang_id).expect("rs theme must exist");
        assert!(rs_theme.token_colors.contains_key("keyword") || rs_theme.token_colors.contains_key("keyword.function"));

        // 4. Line 3: Closing fence
        let t3 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```", &mut state);
        assert!(t3.iter().any(|t| t.2.as_ref() == "punctuation.delimiter"));

        // 5. Line 4: Code block with alias "py"
        let _ = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```py", &mut state);
        let t_py = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "def hello():", &mut state);
        assert!(t_py.iter().any(|t| t.2.as_ref().starts_with("keyword")));
        let _ = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```", &mut state);

        // 6. Code block with "c++"
        let _ = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```c++", &mut state);
        let t_cpp = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "int main() {", &mut state);
        assert!(t_cpp
            .iter()
            .any(|t| t.2.as_ref().starts_with("type") || t.2.as_ref().starts_with("keyword")));
        let _ = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```", &mut state);
    }

    #[test]
    fn test_toml_scm_highlighting_tokens() {
        let toml_src = include_str!("../languages/toml/toml.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("toml", toml_src).expect("toml must compile");
        let tokens = reg.tokenize_line(registered.lang_id, "[package]");
        assert!(tokens.iter().any(|t| t.2.as_ref() == "type" || t.2.as_ref() == "keyword"));

        let kv_tokens = reg.tokenize_line(registered.lang_id, "name = \"ie\"");
        assert!(kv_tokens.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(kv_tokens.iter().any(|t| t.2.as_ref() == "operator"));
        assert!(kv_tokens.iter().any(|t| t.2.as_ref() == "string"));
    }

    #[test]
    fn test_gitcommit_comment_context_tokens() {
        let gitcommit_src = include_str!("../languages/gitcommit/gitcommit.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("gitcommit", gitcommit_src).expect("gitcommit must compile");

        // 1. Branch line
        let branch_tokens = reg.tokenize_line(registered.lang_id, "# On branch main");
        assert!(branch_tokens.iter().any(|t| t.2.as_ref() == "comment"));
        assert!(branch_tokens.iter().any(|t| t.2.as_ref() == "type.branch"));

        // 2. Date line
        let date_tokens = reg.tokenize_line(registered.lang_id, "# Date:   Wed Oct 21 21:06:59 2026 -0500");
        assert!(date_tokens.iter().any(|t| t.2.as_ref() == "comment"));
        assert!(date_tokens.iter().any(|t| t.2.as_ref() == "type.date"));

        // 3. Changes header & file changes
        let changes_header_tokens = reg.tokenize_line(registered.lang_id, "# Changes to be committed:");
        assert!(changes_header_tokens.iter().any(|t| t.2.as_ref() == "type.change"));

        let file_tokens = reg.tokenize_line(registered.lang_id, "#\tmodified:   src/highlight.rs");
        assert!(file_tokens.iter().any(|t| t.2.as_ref() == "type.change"));

        // 4. Standard comment instructions (no branch/date/change)
        let standard_comment = reg.tokenize_line(registered.lang_id, "# Please enter the commit message for your changes.");
        assert!(standard_comment.iter().all(|t| t.2.as_ref() == "comment"));
    }

    #[test]
    fn test_git_rebase_tokens() {
        let rebase_src = include_str!("../languages/gitrebase/gitrebase.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("gitrebase", rebase_src).expect("gitrebase must compile");

        // 1. pick commands with hash and comment (both hex-letter and pure-digit hashes)
        let pick1 = reg.tokenize_line(registered.lang_id, "pick 7467bad # feat: hard reset to include submodules");
        assert!(pick1.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(pick1.iter().any(|t| t.2.as_ref() == "constant"));
        assert!(pick1.iter().any(|t| t.2.as_ref() == "comment"));

        let pick2 = reg.tokenize_line(registered.lang_id, "pick 3605781 # feat: fase 6 completada");
        assert!(pick2.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(pick2.iter().any(|t| t.2.as_ref() == "constant"));
        assert!(pick2.iter().any(|t| t.2.as_ref() == "comment"));

        // 2. drop, squash, exec commands
        let drop_line = reg.tokenize_line(registered.lang_id, "drop bb61410 # feat: add support for first line");
        assert!(drop_line.iter().any(|t| t.2.as_ref() == "keyword.return")); // red
        assert!(drop_line.iter().any(|t| t.2.as_ref() == "constant"));
        assert!(drop_line.iter().any(|t| t.2.as_ref() == "comment"));
        // Ensure "for" in comment is NOT parsed as a keyword!
        assert!(!drop_line.iter().any(|t| t.2.as_ref() == "keyword.repeat"));

        let squash_line = reg.tokenize_line(registered.lang_id, "squash 1c56788 feat: plan");
        assert!(squash_line.iter().any(|t| t.2.as_ref() == "keyword.modifier"));
        assert!(squash_line.iter().any(|t| t.2.as_ref() == "constant"));

        // 3. Documentation comment lines
        let doc_pick = reg.tokenize_line(registered.lang_id, "# p, pick <commit> = use commit");
        assert!(doc_pick.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(doc_pick.iter().any(|t| t.2.as_ref() == "type"));

        let doc_drop = reg.tokenize_line(registered.lang_id, "# d, drop <commit> = remove commit");
        assert!(doc_drop.iter().any(|t| t.2.as_ref() == "keyword.return"));
        assert!(doc_drop.iter().any(|t| t.2.as_ref() == "type"));

        let doc_warn = reg.tokenize_line(registered.lang_id, "# If you remove a line here THAT COMMIT WILL BE LOST.");
        assert!(doc_warn.iter().any(|t| t.2.as_ref() == "keyword.conditional"));

        let doc_rebase = reg.tokenize_line(registered.lang_id, "# Rebase a2b433d..0d48cb1 onto a2b433d (26 commands)");
        assert!(doc_rebase.iter().any(|t| t.2.as_ref() == "constant"));
    }

    #[test]
    fn test_hosts_resolv_conf_port_token() {
        let hosts_src = include_str!("../languages/hosts/hosts.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("hosts", hosts_src).expect("hosts must compile");

        let tokens = reg.tokenize_line(registered.lang_id, "nameserver 1.1.1.1:53");
        assert!(tokens.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(tokens.iter().any(|t| t.2.as_ref() == "type"));
        assert!(tokens.iter().any(|t| t.2.as_ref() == "operator"));
    }

    #[test]
    fn test_ssh_config_tokens() {
        let ssh_src = include_str!("../languages/sshconfig/sshconfig.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("sshconfig", ssh_src).expect("sshconfig must compile");

        let host_tokens = reg.tokenize_line(registered.lang_id, "Host github.com");
        assert!(host_tokens.iter().any(|t| t.2.as_ref() == "keyword.declaration"));
        assert!(host_tokens.iter().any(|t| t.2.as_ref() == "type"));

        let hostname_tokens = reg.tokenize_line(registered.lang_id, "    HostName github.com");
        assert!(hostname_tokens.iter().any(|t| t.2.as_ref() == "property"));
        assert!(hostname_tokens.iter().any(|t| t.2.as_ref() == "type"));

        let user_tokens = reg.tokenize_line(registered.lang_id, "    User git");
        assert!(user_tokens.iter().any(|t| t.2.as_ref() == "keyword.modifier"));
        assert!(user_tokens.iter().any(|t| t.2.as_ref() == "variable.parameter"));

        let id_tokens = reg.tokenize_line(registered.lang_id, "    IdentityFile ~/.ssh/id_ed25519");
        assert!(id_tokens.iter().any(|t| t.2.as_ref() == "keyword.modifier"));
        assert!(id_tokens.iter().any(|t| t.2.as_ref() == "string"));

        let port_tokens = reg.tokenize_line(registered.lang_id, "    Port 2222");
        assert!(port_tokens.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(port_tokens.iter().any(|t| t.2.as_ref() == "number"));
    }

    #[test]
    fn test_crontab_tokens() {
        let cron_src = include_str!("../languages/crontab/crontab.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("crontab", cron_src).expect("crontab must compile");

        // 1. Comment
        let comment_line = reg.tokenize_line(registered.lang_id, "# Edit this file to introduce tasks");
        assert!(comment_line.iter().all(|t| t.2.as_ref() == "comment"));

        // 2. Env assignment
        let env_line = reg.tokenize_line(registered.lang_id, "SHELL=/bin/bash");
        assert!(env_line.iter().any(|t| t.2.as_ref() == "variable"));
        assert!(env_line.iter().any(|t| t.2.as_ref() == "operator"));
        assert!(env_line.iter().any(|t| t.2.as_ref() == "string"));

        // 3. Periodic job line
        let job_line = reg.tokenize_line(registered.lang_id, "0 5 * * 1 /usr/local/bin/backup-system.sh > /dev/null 2>&1");
        assert!(job_line.iter().any(|t| t.2.as_ref() == "number"));
        assert!(job_line.iter().any(|t| t.2.as_ref() == "operator"));
        assert!(job_line.iter().any(|t| t.2.as_ref() == "string"));

        // 4. Special macro
        let macro_line = reg.tokenize_line(registered.lang_id, "@reboot /usr/local/bin/on-boot-tasks.sh");
        assert!(macro_line.iter().any(|t| t.2.as_ref() == "keyword"));
        assert!(macro_line.iter().any(|t| t.2.as_ref() == "string"));
    }

    #[test]
    fn test_html_tag_and_bracket_tokens() {
        let html_src = include_str!("../languages/html/html.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("html", html_src).expect("html must compile");

        // 1. Tag with attribute: <html lang="en">
        let tokens = reg.tokenize_line(registered.lang_id, "<html lang=\"en\">");
        // Verify '<' is punctuation.bracket (darkgray), NOT keyword (blue)
        assert_eq!(tokens[0].0, 0..1);
        assert_eq!(tokens[0].2.as_ref(), "punctuation.bracket");

        // Verify 'html' is keyword (blue)
        assert_eq!(tokens[1].0, 1..5);
        assert_eq!(tokens[1].2.as_ref(), "keyword");

        // Verify 'lang' is property (yellow)
        let lang_token = tokens.iter().find(|t| &"<html lang=\"en\">"[t.0.clone()] == "lang").unwrap();
        assert_eq!(lang_token.2.as_ref(), "property");

        // Verify '=' is operator
        let eq_token = tokens.iter().find(|t| &"<html lang=\"en\">"[t.0.clone()] == "=").unwrap();
        assert_eq!(eq_token.2.as_ref(), "operator");

        // Verify ""en"" is string
        let str_token = tokens.iter().find(|t| &"<html lang=\"en\">"[t.0.clone()] == "\"en\"").unwrap();
        assert_eq!(str_token.2.as_ref(), "string");

        // Verify '>' is punctuation.bracket (darkgray)
        let close_token = tokens.last().unwrap();
        assert_eq!(&"<html lang=\"en\">"[close_token.0.clone()], ">");
        assert_eq!(close_token.2.as_ref(), "punctuation.bracket");

        // 2. Simple tag: <head>
        let head_tokens = reg.tokenize_line(registered.lang_id, "<head>");
        assert_eq!(head_tokens[0].2.as_ref(), "punctuation.bracket");
        assert_eq!(head_tokens[1].2.as_ref(), "keyword");
        assert_eq!(head_tokens[2].2.as_ref(), "punctuation.bracket");

        // 3. Closing tag: </title>
        let close_tokens = reg.tokenize_line(registered.lang_id, "</title>");
        assert_eq!(close_tokens[0].2.as_ref(), "punctuation.bracket"); // </
        assert_eq!(close_tokens[1].2.as_ref(), "keyword"); // title
        assert_eq!(close_tokens[2].2.as_ref(), "punctuation.bracket"); // >

        // 4. Closing tag with invalid attributes: </html sdadw="sdadw">
        let invalid_close_tokens = reg.tokenize_line(registered.lang_id, "</html sdadw=\"sdadw\">");
        assert_eq!(invalid_close_tokens[0].2.as_ref(), "punctuation.bracket"); // </
        assert_eq!(invalid_close_tokens[1].2.as_ref(), "keyword"); // html
        // Verify that sdadw, =, "sdadw" are NOT parsed as property, operator, or string!
        assert!(!invalid_close_tokens.iter().any(|t| t.2.as_ref() == "property"));
        assert!(!invalid_close_tokens.iter().any(|t| t.2.as_ref() == "operator"));
        assert!(!invalid_close_tokens.iter().any(|t| t.2.as_ref() == "string"));
        let last_token = invalid_close_tokens.last().unwrap();
        assert_eq!(&"</html sdadw=\"sdadw\">"[last_token.0.clone()], ">");
        assert_eq!(last_token.2.as_ref(), "punctuation.bracket"); // >
    }

    #[test]
    fn test_rust_highlighting_tokens() {
        let rust_src = include_str!("../languages/rust/rust.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("rust", rust_src).expect("rust must compile");

        // 1. let and mut declarations
        let let_tokens = reg.tokenize_line(registered.lang_id, "let mut counter = 0;");
        assert!(let_tokens.iter().any(|t| t.2.as_ref() == "keyword.declaration"));
        assert!(let_tokens.iter().any(|t| t.2.as_ref() == "keyword.modifier"));
        assert!(let_tokens.iter().any(|t| t.2.as_ref() == "number"));

        // 2. use statement
        let use_tokens = reg.tokenize_line(registered.lang_id, "use std::collections::HashMap;");
        assert!(use_tokens.iter().any(|t| t.2.as_ref() == "keyword.import"));
        assert!(use_tokens.iter().any(|t| t.2.as_ref() == "type"));

        // 3. #[test] attribute
        let attr_tokens = reg.tokenize_line(registered.lang_id, "#[test]");
        assert!(attr_tokens.iter().any(|t| t.2.as_ref() == "punctuation.special"));
        assert!(attr_tokens.iter().any(|t| t.2.as_ref() == "attribute"));

        // 4. if let Some(x) = res
        let if_let_tokens = reg.tokenize_line(registered.lang_id, "if let Some(x) = res {");
        assert!(if_let_tokens.iter().any(|t| t.2.as_ref() == "keyword.conditional"));
        assert!(if_let_tokens.iter().any(|t| t.2.as_ref() == "keyword.declaration"));
        assert!(if_let_tokens.iter().any(|t| t.2.as_ref() == "type")); // Some is yellow type!
        assert!(!if_let_tokens.iter().any(|t| t.2.as_ref() == "function.call")); // not function.call!

        // 5. match, None, Result, Ok, Err
        let match_tokens = reg.tokenize_line(registered.lang_id, "match opt { None => Ok(1), _ => Err(0) }");
        assert!(match_tokens.iter().any(|t| t.2.as_ref() == "keyword.conditional"));
        let type_count = match_tokens.iter().filter(|t| t.2.as_ref() == "type").count();
        assert!(type_count >= 3); // None, Ok, Err all typed!

        let result_tokens = reg.tokenize_line(registered.lang_id, "fn run() -> Result<(), Error>;");
        assert!(result_tokens.iter().any(|t| t.2.as_ref() == "keyword.function"));
        assert!(result_tokens.iter().any(|t| t.2.as_ref() == "function.definition"));
        assert!(result_tokens.iter().any(|t| t.2.as_ref() == "type"));

        // 6. Raw strings (single-line with internal quotes and multi-line stateful)
        let raw_tokens = reg.tokenize_line(registered.lang_id, "let s = r#\"hello \"world\"\"#;");
        assert!(raw_tokens.iter().any(|t| t.2.as_ref() == "string"));

        let mut state = LexState::default();
        let ml_line1 = reg.tokenize_line_stateful(registered.lang_id, "let s = r#\"first line", &mut state);
        let ml_line2 = reg.tokenize_line_stateful(registered.lang_id, "second \"line\"", &mut state);
        let ml_line3 = reg.tokenize_line_stateful(registered.lang_id, "\"#;", &mut state);
        assert!(ml_line1.iter().any(|t| t.2.as_ref() == "string"));
        assert!(ml_line2.iter().any(|t| t.2.as_ref() == "string"));
        assert!(ml_line3.iter().any(|t| t.2.as_ref() == "string"));

        // 7. Theme colors check: keywords in magenta, types in yellow
        let theme_str = include_str!("../languages/rust/theme.json");
        assert!(theme_str.contains("\"keyword.conditional\": \"magenta\""));
        assert!(theme_str.contains("\"keyword.declaration\": \"magenta\""));
        assert!(theme_str.contains("\"keyword.modifier\": \"magenta\""));
        assert!(theme_str.contains("\"keyword.import\": \"magenta\""));
        assert!(theme_str.contains("\"type\": \"yellow\""));
    }

    #[test]
    fn test_user_config_overrides_builtin_theme_and_rules() {
        let test_dir = std::env::temp_dir().join("ie_test_override_precedence");
        let _ = fs::remove_dir_all(&test_dir);
        let _ = fs::create_dir_all(&test_dir);

        // 1. Override built-in Rust theme via rust/theme.json
        let rust_dir = test_dir.join("rust");
        let _ = fs::create_dir_all(&rust_dir);
        let _ = fs::write(
            rust_dir.join("theme.json"),
            r#"{"keyword": "red", "custom_tag": "blue"}"#,
        );

        // 2. Override built-in Python via flat python.json
        let _ = fs::write(
            test_dir.join("python.json"),
            r#"{"keyword": "yellow"}"#,
        );

        // 3. Override global fallback theme via global_theme.json
        let _ = fs::write(
            test_dir.join("global_theme.json"),
            r#"{"fallback_token": "green"}"#,
        );

        // 4. Add a brand new language via zig/zig.toml
        let zig_dir = test_dir.join("zig");
        let _ = fs::create_dir_all(&zig_dir);
        let _ = fs::write(
            zig_dir.join("zig.toml"),
            "[meta]\nname = \"zig\"\nextensions = [\"zig\"]\n\n[states.root]\nrules = [\n    { match = '\\\\b(?:fn|pub)\\\\b', token = \"keyword\" },\n]\n",
        );

        let highlighter = init_highlighter(&test_dir);

        // Verify Rust theme was overridden
        let rust_lang = highlighter.languages.get("rust").expect("rust exists");
        assert_eq!(rust_lang.theme.color_for("keyword"), Some(Color::Red));
        assert_eq!(rust_lang.theme.color_for("custom_tag"), Some(Color::Blue));

        // Verify Python theme was overridden via flat json
        let python_lang = highlighter.languages.get("python").expect("python exists");
        assert_eq!(python_lang.theme.color_for("keyword"), Some(Color::Yellow));

        // Verify global theme
        assert_eq!(highlighter.global_theme.color_for("fallback_token"), Some(Color::Green));

        // Verify newly added language
        assert!(highlighter.languages.contains_key("zig"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_js_template_string_interpolation_and_closing() {
        let js_src = include_str!("../languages/javascript/javascript.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("javascript", js_src).expect("javascript must compile");

        let mut state = LexState::DOC_START;
        let line1 = "output.textContent = `[${res.timestamp}] Status: ${res.status} (latency: ${res.latency}ms)`;";
        let tokens1 = reg.tokenize_line_stateful(registered.lang_id, line1, &mut state);

        // Verify template expressions were parsed and closed
        assert!(tokens1.iter().any(|t| t.2.as_ref() == "punctuation.special"));
        assert!(tokens1.iter().any(|t| t.2.as_ref() == "string"));

        // Next line should be back at root state and not treated as string!
        let line2 = "console.log(\"Health check recorded:\", res);";
        let tokens2 = reg.tokenize_line_stateful(registered.lang_id, line2, &mut state);
        assert!(tokens2.iter().any(|t| t.2.as_ref() == "type.builtin")); // console
        assert!(tokens2.iter().any(|t| t.2.as_ref() == "function.call")); // log
    }

    #[test]
    fn test_ts_template_string_interpolation_and_closing() {
        let ts_src = include_str!("../languages/typescript/typescript.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("typescript", ts_src).expect("typescript must compile");

        let mut state = LexState::DOC_START;
        let line1 = "const msg: string = `Status: ${config.enabled ? \"ON\" : \"OFF\"} (port: ${config.port})`;";
        let tokens1 = reg.tokenize_line_stateful(registered.lang_id, line1, &mut state);

        assert!(tokens1.iter().any(|t| t.2.as_ref() == "punctuation.special"));
        assert!(tokens1.iter().any(|t| t.2.as_ref() == "string"));

        // Next line should be back at root state
        let line2 = "export function run(): void {}";
        let tokens2 = reg.tokenize_line_stateful(registered.lang_id, line2, &mut state);
        assert!(tokens2.iter().any(|t| t.2.as_ref() == "keyword.import")); // export
        assert!(tokens2.iter().any(|t| t.2.as_ref() == "keyword.function")); // function
    }

    #[test]
    fn test_c_cpp_preprocessor_highlighting_tokens() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);

        // Test C
        let c_id = highlighter.registry().lang_id("c").expect("c in registry");
        let mut state = LexState::default();

        // 1. #include <stdio.h>
        let tokens = highlighter.registry().tokenize_line_stateful(c_id, "#include <stdio.h>", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.directive"); // #include
        assert_eq!(tokens[1].2.as_ref(), "string"); // <stdio.h>

        // 2. #define BUFFER_CAPACITY 1024
        let mut state = LexState::default();
        let tokens = highlighter.registry().tokenize_line_stateful(c_id, "#define BUFFER_CAPACITY 1024", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.directive"); // #define
        assert_eq!(tokens[1].2.as_ref(), "constant"); // BUFFER_CAPACITY
        assert_eq!(tokens[2].2.as_ref(), "number"); // 1024

        // 3. #  define FORCE_INLINE inline __attribute__((always_inline))
        let mut state = LexState::default();
        let tokens = highlighter.registry().tokenize_line_stateful(c_id, "#  define FORCE_INLINE inline __attribute__((always_inline))", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.directive"); // #  define
        assert_eq!(tokens[1].2.as_ref(), "constant"); // FORCE_INLINE
        assert_eq!(tokens[2].2.as_ref(), "keyword.modifier"); // inline
        assert_eq!(tokens[3].2.as_ref(), "keyword.modifier"); // __attribute__

        // 4. #if defined(__GNUC__) || defined(__clang__)
        let mut state = LexState::default();
        let tokens = highlighter.registry().tokenize_line_stateful(c_id, "#if defined(__GNUC__) || defined(__clang__)", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.directive"); // #if
        assert_eq!(tokens[1].2.as_ref(), "keyword.directive"); // defined
        assert_eq!(tokens[3].2.as_ref(), "variable"); // __GNUC__
        assert_eq!(tokens[5].2.as_ref(), "operator"); // ||
        assert_eq!(tokens[6].2.as_ref(), "keyword.directive"); // defined
        assert_eq!(tokens[8].2.as_ref(), "variable"); // __clang__

        // Test C++
        let cpp_id = highlighter.registry().lang_id("cpp").expect("cpp in registry");
        let mut state = LexState::default();
        let tokens = highlighter.registry().tokenize_line_stateful(cpp_id, "#include <iostream>", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.directive"); // #include
        assert_eq!(tokens[1].2.as_ref(), "string"); // <iostream>
    }

    #[test]
    fn test_caddyfile_highlighting_tokens() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);
        let caddy_id = highlighter.registry().lang_id("caddy").expect("caddy in registry");

        // 1. Domains: example.com, www.example.com {
        let mut state = LexState::default();
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "example.com, www.example.com {", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "type"); // example.com
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // ,
        assert_eq!(tokens[2].2.as_ref(), "type"); // www.example.com
        assert_eq!(tokens[3].2.as_ref(), "punctuation.bracket"); // {

        // 2. root * /var/www/html
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "    root * /var/www/html", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // root
        assert_eq!(tokens[1].2.as_ref(), "operator"); // *
        assert_eq!(tokens[2].2.as_ref(), "string"); // /var/www/html

        // 3. encode gzip zstd
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "    encode gzip zstd", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // encode
        assert_eq!(tokens[1].2.as_ref(), "constant.builtin"); // gzip
        assert_eq!(tokens[2].2.as_ref(), "constant.builtin"); // zstd

        // 4. Matcher definition: @api {
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "    @api {", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "type"); // @api
        assert_eq!(tokens[1].2.as_ref(), "punctuation.bracket"); // {

        // 5. Matcher directive: path /api/*
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "        path /api/*", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // path
        assert_eq!(tokens[1].2.as_ref(), "string"); // /api/*

        // 6. reverse_proxy @api localhost:8080
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "    reverse_proxy @api localhost:8080", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // reverse_proxy
        assert_eq!(tokens[1].2.as_ref(), "type"); // @api
        assert_eq!(tokens[2].2.as_ref(), "type"); // localhost:8080

        // 7. Placeholders inside string: respond "{http.error.status_code} {http.error.status_text}"
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "        respond \"{http.error.status_code} {http.error.status_text}\"", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // respond
        assert_eq!(tokens[1].2.as_ref(), "string"); // "
        assert_eq!(tokens[2].2.as_ref(), "variable"); // {http.error.status_code}
        assert_eq!(tokens[4].2.as_ref(), "variable"); // {http.error.status_text}
        assert_eq!(tokens[5].2.as_ref(), "string"); // "

        // 8. Log directive and output file path: output file /var/log/caddy/access.log
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "        output file /var/log/caddy/access.log", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword.modifier"); // output
        assert_eq!(tokens[1].2.as_ref(), "constant.builtin"); // file
        assert_eq!(tokens[2].2.as_ref(), "string"); // /var/log/caddy/access.log

        // 9. Port block and respond status code: :8081 { respond "OK" 200 }
        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, ":8081 {", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "type"); // :8081
        assert_eq!(tokens[1].2.as_ref(), "punctuation.bracket"); // {

        let tokens = highlighter.registry().tokenize_line_stateful(caddy_id, "    respond \"OK\" 200", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // respond
        assert_eq!(tokens[1].2.as_ref(), "string"); // "OK"
        assert_eq!(tokens[2].2.as_ref(), "constant.numeric"); // 200
    }

    #[test]
    fn test_yaml_highlighting_tokens() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);
        let yaml_id = highlighter.registry().lang_id("yaml").expect("yaml in registry");
        let mut state = LexState::default();

        // 1. Header delimiter: ---
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "---", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "punctuation.delimiter");

        // 2. apiVersion: apps/v1
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "apiVersion: apps/v1", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // apiVersion
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :

        // 3. Namespaced key: app.kubernetes.io/name: ie-gateway
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "    app.kubernetes.io/name: ie-gateway", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // app.kubernetes.io/name
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :

        // 4. Anchors & aliases: defaults: &resource_defaults
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "defaults: &resource_defaults", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // defaults
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :
        assert_eq!(tokens[2].2.as_ref(), "variable"); // &resource_defaults

        // 5. Merge key: <<: *resource_defaults
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "            <<: *resource_defaults", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "operator"); // <<
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :
        assert_eq!(tokens[2].2.as_ref(), "variable"); // *resource_defaults

        // 6. Numbers with units: memory: 2Gi
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "    memory: 2Gi", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // memory
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :
        assert_eq!(tokens[2].2.as_ref(), "number"); // 2Gi

        // 7. Percentages: maxSurge: 25%
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "      maxSurge: 25%", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // maxSurge
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :
        assert_eq!(tokens[2].2.as_ref(), "number"); // 25%

        // 8. List bullet item: - name: http
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "        - name: http", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "operator"); // -
        assert_eq!(tokens[1].2.as_ref(), "keyword"); // name
        assert_eq!(tokens[2].2.as_ref(), "punctuation.delimiter"); // :

        // 9. Multi-line folded scalar: description: >
        let tokens = highlighter.registry().tokenize_line_stateful(yaml_id, "description: >", &mut state);
        assert_eq!(tokens[0].2.as_ref(), "keyword"); // description
        assert_eq!(tokens[1].2.as_ref(), "punctuation.delimiter"); // :
        assert_eq!(tokens[2].2.as_ref(), "operator"); // >
    }

    #[test]
    fn test_sample_files_tokenization() {
        let dummy_dir = PathBuf::from("/tmp/nonexistent_ie_test_dir_123");
        let highlighter = init_highlighter(&dummy_dir);

        let sample_files = [
            ("samples/sources.list", "apt_sources"),
            ("samples/fedora.repo", "apt_sources"),
            ("samples/main.c", "c"),
            ("samples/Program.cs", "c_sharp"),
            ("samples/Caddyfile", "caddy"),
            ("samples/server.cpp", "cpp"),
            ("samples/crontab", "crontab"),
            ("samples/style.css", "css"),
            ("samples/app.desktop", "desktop"),
            ("samples/example.patch", "diff"),
            ("samples/Dockerfile", "dockerfile"),
            ("samples/.env", "env"),
            ("samples/fstab", "fstab"),
            ("samples/COMMIT_EDITMSG", "gitcommit"),
            ("samples/.gitconfig", "gitconfig"),
            ("samples/.gitignore", "gitignore"),
            ("samples/git-rebase-todo", "gitrebase"),
            ("samples/main.go", "go"),
            ("samples/hosts", "hosts"),
            ("samples/resolv.conf", "hosts"),
            ("samples/index.html", "html"),
            ("samples/config.ini", "ini"),
            ("samples/Application.java", "java"),
            ("samples/app.js", "javascript"),
            ("samples/data.json", "json"),
            ("samples/Main.kt", "kotlin"),
            ("samples/script.lua", "lua"),
            ("samples/Makefile", "makefile"),
            ("samples/codeblocks.md", "markdown"),
            ("samples/nginx.conf", "nginx"),
            ("samples/script.py", "python"),
            ("samples/main.rs", "rust"),
            ("samples/deploy.sh", "shell"),
            ("samples/query.sql", "sql"),
            ("samples/ssh_config", "sshconfig"),
            ("samples/sudoers", "sudoers"),
            ("samples/sudoers.tmp", "sudoers"),
            ("samples/override.conf", "systemd"),
            ("samples/docker.service", "systemd"),
            ("samples/config.toml", "toml"),
            ("samples/app.ts", "typescript"),
            ("samples/config.yaml", "yaml"),
        ];

        for (file_path, expected_lang) in sample_files {
            let path = Path::new(file_path);
            let detected = detect_language(path);
            assert_eq!(detected, Some(expected_lang), "Failed detecting language for {}", file_path);

            let content = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read sample file {}: {}", file_path, e));

            let lang_id = highlighter.registry().lang_id(expected_lang)
                .unwrap_or_else(|| panic!("Language {} not in registry", expected_lang));

            let mut state = LexState::default();
            for line in content.lines() {
                let _tokens = highlighter.registry().tokenize_line_stateful(lang_id, line, &mut state);
            }
        }
    }
}
