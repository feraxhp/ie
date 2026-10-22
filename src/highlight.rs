use std::fs;
use std::path::Path;
use texera::{include_languaje_dir, HighlightPlugin, LanguageToml, ThemeMap};
#[cfg(test)]
use texera::{LanguageRegistry, LexState};

/// Detects the language or syntax format from the given file name or extension.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    // 1. Exact file name detection
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
            _ => {}
        }
        if lower.starts_with("git-rebase") {
            return Some("gitrebase");
        }
        if lower.starts_with(".git") {
            return Some("gitconfig");
        }
        if lower.starts_with(".env.") || lower.ends_with(".env") {
            return Some("env");
        }
        if lower == "config" {
            if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
                if parent.eq_ignore_ascii_case(".ssh") || parent.eq_ignore_ascii_case("ssh") {
                    return Some("sshconfig");
                }
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

/// Loads a language's TOML specification on demand for codeblock syntax highlighting.
/// Reuses `detect_language` to resolve aliases or extensions to canonical language grammars.
/// Checks user configuration directory first, then falls back to built-in grammars.
pub fn load_codeblock_toml(tag: &str, config_dir: &Path) -> Option<String> {
    let clean = tag.trim();
    if clean.is_empty() {
        return None;
    }

    // Resolve canonical language identifier using detect_language
    let lang = resolve_codeblock_language(clean).unwrap_or(clean);

    // 1. Check user config overrides first
    if config_dir.exists() && config_dir.is_dir() {
        let candidates = [
            config_dir.join(lang).join(format!("{}.toml", lang)),
            config_dir.join(lang).join("syntax.toml"),
            config_dir.join(format!("{}.toml", lang)),
            config_dir.join(clean).join(format!("{}.toml", clean)),
            config_dir.join(format!("{}.toml", clean)),
        ];

        for candidate in &candidates {
            if candidate.is_file() {
                if let Ok(content) = fs::read_to_string(candidate) {
                    return Some(content);
                }
            }
        }
    }

    // 2. Fall back to built-in language definitions
    get_builtin_toml(lang)
        .or_else(|| get_builtin_toml(clean))
        .map(|s| s.to_string())
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
        ("variable", ratatui::style::Color::White),
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
pub fn apply_config_overrides(highlight: &mut HighlightPlugin, config_dir: &Path) {
    if !config_dir.exists() || !config_dir.is_dir() {
        return;
    }

    // 1. Root global theme: global_theme.json or theme.json
    for global_name in &["global_theme.json", "theme.json"] {
        let global_path = config_dir.join(global_name);
        if global_path.is_file() {
            if let Ok(theme) = ThemeMap::load_from_file(&global_path) {
                for (token, color) in theme.token_colors {
                    highlight.global_theme.set_color(token, color);
                }
                break;
            }
        }
    }

    // 2. Scan entries in config_dir
    let Ok(entries) = fs::read_dir(config_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Directory-based language: config_dir/{lang}/
            let Some(lang_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let mut toml_content: Option<String> = None;
            let mut user_theme: Option<ThemeMap> = None;

            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        if let Some(ext) = sub_path.extension().and_then(|e| e.to_str()) {
                            if ext == "toml" && toml_content.is_none() {
                                if let Ok(content) = fs::read_to_string(&sub_path) {
                                    toml_content = Some(content);
                                }
                            } else if ext == "json" && user_theme.is_none() {
                                if let Ok(theme) = ThemeMap::load_from_file(&sub_path) {
                                    user_theme = Some(theme);
                                }
                            }
                        }
                    }
                }
            }

            override_or_insert_language(highlight, lang_name, toml_content, user_theme);
        } else if path.is_file() {
            // Flat-file based: config_dir/{lang}.toml and config_dir/{lang}.json
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
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
/// Installs an on-demand language loader callback for Markdown codeblock syntax injection.
pub fn init_highlighter(config_dir: &Path) -> HighlightPlugin {
    // 1. Load built-in project languages using the Texera proc-macro
    let mut highlight = include_languaje_dir!("languages");

    // Initialize global fallback theme for embedded languages
    init_global_theme(&mut highlight.global_theme);

    // 2. Load dynamic specs with strict precedence: config_dir overrides built-ins
    apply_config_overrides(&mut highlight, config_dir);

    // 3. Register on-demand loader for codeblocks, resolving languages via detect_language
    let config_dir_buf = config_dir.to_path_buf();
    highlight = highlight.with_loader(move |name: &str| {
        load_codeblock_toml(name, &config_dir_buf)
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
    }

    #[test]
    fn test_detect_git_files() {
        assert_eq!(detect_language(Path::new(".gitignore")), Some("gitignore"));
        assert_eq!(detect_language(Path::new(".gitattributes")), Some("gitignore"));
        assert_eq!(detect_language(Path::new(".gitmodules")), Some("gitignore"));
        assert_eq!(detect_language(Path::new("COMMIT_EDITMSG")), Some("gitcommit"));
        assert_eq!(detect_language(Path::new(".gitconfig")), Some("gitconfig"));
        assert_eq!(detect_language(Path::new("gitconfig")), Some("gitconfig"));
        assert_eq!(detect_language(Path::new("git-rebase-todo")), Some("gitrebase"));
        assert_eq!(detect_language(Path::new(".git/rebase-merge/git-rebase-todo")), Some("gitrebase"));
        assert_eq!(detect_language(Path::new("todo.rebase")), Some("gitrebase"));
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
        assert_eq!(resolve_codeblock_language("unknown_lang_123"), None);
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
        assert!(t0.iter().any(|t| t.1.as_ref() == "markup.heading"));

        // 2. Line 1: Opening code fence with alias "rs"
        let t1 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```rs", &mut state);
        assert!(t1.iter().any(|t| t.1.as_ref() == "punctuation.delimiter"));

        // 3. Line 2: Rust code inside the block
        let t2 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "fn main() {", &mut state);
        assert!(t2.iter().any(|t| t.1.as_ref().starts_with("keyword")));

        // 4. Line 3: Closing fence
        let t3 = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```", &mut state);
        assert!(t3.iter().any(|t| t.1.as_ref() == "punctuation.delimiter"));

        // 5. Line 4: Code block with alias "py"
        let _ = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "```py", &mut state);
        let t_py = highlighter
            .registry()
            .tokenize_line_stateful(md_lang_id, "def hello():", &mut state);
        assert!(t_py.iter().any(|t| t.1.as_ref().starts_with("keyword")));
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
            .any(|t| t.1.as_ref().starts_with("type") || t.1.as_ref().starts_with("keyword")));
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
        assert!(tokens.iter().any(|t| t.1.as_ref() == "type" || t.1.as_ref() == "keyword"));

        let kv_tokens = reg.tokenize_line(registered.lang_id, "name = \"ie\"");
        assert!(kv_tokens.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(kv_tokens.iter().any(|t| t.1.as_ref() == "operator"));
        assert!(kv_tokens.iter().any(|t| t.1.as_ref() == "string"));
    }

    #[test]
    fn test_gitcommit_comment_context_tokens() {
        let gitcommit_src = include_str!("../languages/gitcommit/gitcommit.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("gitcommit", gitcommit_src).expect("gitcommit must compile");

        // 1. Branch line
        let branch_tokens = reg.tokenize_line(registered.lang_id, "# On branch main");
        assert!(branch_tokens.iter().any(|t| t.1.as_ref() == "comment"));
        assert!(branch_tokens.iter().any(|t| t.1.as_ref() == "type.branch"));

        // 2. Date line
        let date_tokens = reg.tokenize_line(registered.lang_id, "# Date:   Wed Oct 21 21:06:59 2026 -0500");
        assert!(date_tokens.iter().any(|t| t.1.as_ref() == "comment"));
        assert!(date_tokens.iter().any(|t| t.1.as_ref() == "type.date"));

        // 3. Changes header & file changes
        let changes_header_tokens = reg.tokenize_line(registered.lang_id, "# Changes to be committed:");
        assert!(changes_header_tokens.iter().any(|t| t.1.as_ref() == "type.change"));

        let file_tokens = reg.tokenize_line(registered.lang_id, "#\tmodified:   src/highlight.rs");
        assert!(file_tokens.iter().any(|t| t.1.as_ref() == "type.change"));

        // 4. Standard comment instructions (no branch/date/change)
        let standard_comment = reg.tokenize_line(registered.lang_id, "# Please enter the commit message for your changes.");
        assert!(standard_comment.iter().all(|t| t.1.as_ref() == "comment"));
    }

    #[test]
    fn test_git_rebase_tokens() {
        let rebase_src = include_str!("../languages/gitrebase/gitrebase.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("gitrebase", rebase_src).expect("gitrebase must compile");

        // 1. pick commands with hash and comment (both hex-letter and pure-digit hashes)
        let pick1 = reg.tokenize_line(registered.lang_id, "pick 7467bad # feat: hard reset to include submodules");
        assert!(pick1.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(pick1.iter().any(|t| t.1.as_ref() == "constant"));
        assert!(pick1.iter().any(|t| t.1.as_ref() == "comment"));

        let pick2 = reg.tokenize_line(registered.lang_id, "pick 3605781 # feat: fase 6 completada");
        assert!(pick2.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(pick2.iter().any(|t| t.1.as_ref() == "constant"));
        assert!(pick2.iter().any(|t| t.1.as_ref() == "comment"));

        // 2. drop, squash, exec commands
        let drop_line = reg.tokenize_line(registered.lang_id, "drop bb61410 # feat: add support for first line");
        assert!(drop_line.iter().any(|t| t.1.as_ref() == "keyword.return")); // red
        assert!(drop_line.iter().any(|t| t.1.as_ref() == "constant"));
        assert!(drop_line.iter().any(|t| t.1.as_ref() == "comment"));
        // Ensure "for" in comment is NOT parsed as a keyword!
        assert!(!drop_line.iter().any(|t| t.1.as_ref() == "keyword.repeat"));

        let squash_line = reg.tokenize_line(registered.lang_id, "squash 1c56788 feat: plan");
        assert!(squash_line.iter().any(|t| t.1.as_ref() == "keyword.modifier"));
        assert!(squash_line.iter().any(|t| t.1.as_ref() == "constant"));

        // 3. Documentation comment lines
        let doc_pick = reg.tokenize_line(registered.lang_id, "# p, pick <commit> = use commit");
        assert!(doc_pick.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(doc_pick.iter().any(|t| t.1.as_ref() == "type"));

        let doc_drop = reg.tokenize_line(registered.lang_id, "# d, drop <commit> = remove commit");
        assert!(doc_drop.iter().any(|t| t.1.as_ref() == "keyword.return"));
        assert!(doc_drop.iter().any(|t| t.1.as_ref() == "type"));

        let doc_warn = reg.tokenize_line(registered.lang_id, "# If you remove a line here THAT COMMIT WILL BE LOST.");
        assert!(doc_warn.iter().any(|t| t.1.as_ref() == "keyword.conditional"));

        let doc_rebase = reg.tokenize_line(registered.lang_id, "# Rebase a2b433d..0d48cb1 onto a2b433d (26 commands)");
        assert!(doc_rebase.iter().any(|t| t.1.as_ref() == "constant"));
    }

    #[test]
    fn test_hosts_resolv_conf_port_token() {
        let hosts_src = include_str!("../languages/hosts/hosts.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("hosts", hosts_src).expect("hosts must compile");

        let tokens = reg.tokenize_line(registered.lang_id, "nameserver 1.1.1.1:53");
        assert!(tokens.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(tokens.iter().any(|t| t.1.as_ref() == "type"));
        assert!(tokens.iter().any(|t| t.1.as_ref() == "operator"));
    }

    #[test]
    fn test_ssh_config_tokens() {
        let ssh_src = include_str!("../languages/sshconfig/sshconfig.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("sshconfig", ssh_src).expect("sshconfig must compile");

        let host_tokens = reg.tokenize_line(registered.lang_id, "Host github.com");
        assert!(host_tokens.iter().any(|t| t.1.as_ref() == "keyword.declaration"));
        assert!(host_tokens.iter().any(|t| t.1.as_ref() == "type"));

        let hostname_tokens = reg.tokenize_line(registered.lang_id, "    HostName github.com");
        assert!(hostname_tokens.iter().any(|t| t.1.as_ref() == "property"));
        assert!(hostname_tokens.iter().any(|t| t.1.as_ref() == "type"));

        let user_tokens = reg.tokenize_line(registered.lang_id, "    User git");
        assert!(user_tokens.iter().any(|t| t.1.as_ref() == "keyword.modifier"));
        assert!(user_tokens.iter().any(|t| t.1.as_ref() == "variable.parameter"));

        let id_tokens = reg.tokenize_line(registered.lang_id, "    IdentityFile ~/.ssh/id_ed25519");
        assert!(id_tokens.iter().any(|t| t.1.as_ref() == "keyword.modifier"));
        assert!(id_tokens.iter().any(|t| t.1.as_ref() == "string"));

        let port_tokens = reg.tokenize_line(registered.lang_id, "    Port 2222");
        assert!(port_tokens.iter().any(|t| t.1.as_ref() == "keyword"));
        assert!(port_tokens.iter().any(|t| t.1.as_ref() == "number"));
    }

    #[test]
    fn test_rust_highlighting_tokens() {
        let rust_src = include_str!("../languages/rust/rust.toml");
        let reg = LanguageRegistry::new();
        let registered = reg.register("rust", rust_src).expect("rust must compile");

        // 1. let and mut declarations
        let let_tokens = reg.tokenize_line(registered.lang_id, "let mut counter = 0;");
        assert!(let_tokens.iter().any(|t| t.1.as_ref() == "keyword.declaration"));
        assert!(let_tokens.iter().any(|t| t.1.as_ref() == "keyword.modifier"));
        assert!(let_tokens.iter().any(|t| t.1.as_ref() == "number"));

        // 2. use statement
        let use_tokens = reg.tokenize_line(registered.lang_id, "use std::collections::HashMap;");
        assert!(use_tokens.iter().any(|t| t.1.as_ref() == "keyword.import"));
        assert!(use_tokens.iter().any(|t| t.1.as_ref() == "type"));

        // 3. #[test] attribute
        let attr_tokens = reg.tokenize_line(registered.lang_id, "#[test]");
        assert!(attr_tokens.iter().any(|t| t.1.as_ref() == "punctuation.special"));
        assert!(attr_tokens.iter().any(|t| t.1.as_ref() == "attribute"));

        // 4. if let Some(x) = res
        let if_let_tokens = reg.tokenize_line(registered.lang_id, "if let Some(x) = res {");
        assert!(if_let_tokens.iter().any(|t| t.1.as_ref() == "keyword.conditional"));
        assert!(if_let_tokens.iter().any(|t| t.1.as_ref() == "keyword.declaration"));
        assert!(if_let_tokens.iter().any(|t| t.1.as_ref() == "type")); // Some is yellow type!
        assert!(!if_let_tokens.iter().any(|t| t.1.as_ref() == "function.call")); // not function.call!

        // 5. match, None, Result, Ok, Err
        let match_tokens = reg.tokenize_line(registered.lang_id, "match opt { None => Ok(1), _ => Err(0) }");
        assert!(match_tokens.iter().any(|t| t.1.as_ref() == "keyword.conditional"));
        let type_count = match_tokens.iter().filter(|t| t.1.as_ref() == "type").count();
        assert!(type_count >= 3); // None, Ok, Err all typed!

        let result_tokens = reg.tokenize_line(registered.lang_id, "fn run() -> Result<(), Error>;");
        assert!(result_tokens.iter().any(|t| t.1.as_ref() == "keyword.function"));
        assert!(result_tokens.iter().any(|t| t.1.as_ref() == "function.definition"));
        assert!(result_tokens.iter().any(|t| t.1.as_ref() == "type"));

        // 6. Raw strings (single-line with internal quotes and multi-line stateful)
        let raw_tokens = reg.tokenize_line(registered.lang_id, "let s = r#\"hello \"world\"\"#;");
        assert!(raw_tokens.iter().any(|t| t.1.as_ref() == "string"));

        let mut state = LexState::default();
        let ml_line1 = reg.tokenize_line_stateful(registered.lang_id, "let s = r#\"first line", &mut state);
        let ml_line2 = reg.tokenize_line_stateful(registered.lang_id, "second \"line\"", &mut state);
        let ml_line3 = reg.tokenize_line_stateful(registered.lang_id, "\"#;", &mut state);
        assert!(ml_line1.iter().any(|t| t.1.as_ref() == "string"));
        assert!(ml_line2.iter().any(|t| t.1.as_ref() == "string"));
        assert!(ml_line3.iter().any(|t| t.1.as_ref() == "string"));

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
            "[meta]\nname = \"zig\"\nextensions = [\"zig\"]\n\n[states.root]\nrules = [\n    { match = '\\b(?:fn|pub)\\b', token = \"keyword\" },\n]\n",
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
}
