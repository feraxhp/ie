use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use mynox::Error;

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
}
