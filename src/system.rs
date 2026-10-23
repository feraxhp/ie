use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use mynox::Error;

pub struct File;

impl File {
    pub fn read(path: &Path) -> Result<String, Error> {
        read_to_string(path).map_err(|e| {
            mynox::error!(
                "fs::read",
                "Failed to read file",
                format!("Could not read `{}`: {}", path.display(), e)
                =>
                "Ensure the file exists and you have **read permissions**"
            )
        })
    }

    pub fn write(path: &Path, contents: &str) -> Result<(), Error> {
        write(path, contents).map_err(|e| {
            mynox::error!(
                "fs::write",
                "Failed to write file",
                format!("Could not write to `{}`: {}", path.display(), e)
                =>
                "Check available disk space and **write permissions**"
            )
        })
    }
}

pub struct Directories;

impl Directories {
    pub fn new() -> Result<ProjectDirs, Error> {
        ProjectDirs::from("", "", "ie").ok_or_else(|| {
            mynox::error!(
                "fs::directories",
                "Internal system error",
                "Could not determine system **directories** for the current platform"
            )
        })
    }

    /// Returns the cross-platform configuration directory for dynamic language specs.
    /// Linux: ~/.config/ie/languages
    /// Windows: %APPDATA%\ie\languages
    /// macOS: ~/Library/Application Support/ie/languages
    pub fn languages_dir() -> Result<PathBuf, Error> {
        let pd = Self::new()?;
        let dir = pd.config_dir().join("languages");
        Self::ensure_dir(&dir)?;
        Ok(dir)
    }

    pub fn ensure_dir(location: &Path) -> Result<(), Error> {
        if !location.exists() {
            create_dir_all(location).map_err(|e| {
                mynox::error!(
                    "fs::create_dir",
                    "Failed to create configuration directory",
                    format!("Could not create directory `{}`: {}", location.display(), e)
                    =>
                    format!("Ensure parent directory exists and has **write permissions**")
                )
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_directories() {
        let pd = Directories::new();
        assert!(pd.is_ok());

        let lang_dir = Directories::languages_dir();
        assert!(lang_dir.is_ok());
        let path = lang_dir.unwrap();
        assert!(path.ends_with("ie/languages") || path.ends_with("ie\\languages"));
        assert!(path.exists());
    }

    #[test]
    fn test_file_read_write() {
        let temp_file = std::env::temp_dir().join(format!("ie_file_test_{}.txt", std::process::id()));
        let content = "hello world from grp-style file read";
        let res = File::write(&temp_file, content);
        assert!(res.is_ok());

        let read_res = File::read(&temp_file);
        assert!(read_res.is_ok());
        assert_eq!(read_res.unwrap(), content);

        let _ = std::fs::remove_file(&temp_file);
    }
}
