use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use mynox::Error;

use crate::system::File;

pub struct Cache;

impl Cache {
    /// Returns the platform-specific temporary directory.
    /// Linux & macOS: /tmp
    /// Windows: %TEMP% (std::env::temp_dir())
    pub fn temp_dir() -> PathBuf {
        #[cfg(unix)]
        {
            PathBuf::from("/tmp")
        }
        #[cfg(not(unix))]
        {
            std::env::temp_dir()
        }
    }

    /// Resolves the canonical or absolute path of the target file.
    pub fn canonical_path(target: &Path) -> PathBuf {
        if target.exists() {
            target.canonicalize().unwrap_or_else(|_| {
                if target.is_absolute() {
                    target.to_path_buf()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(target))
                        .unwrap_or_else(|_| target.to_path_buf())
                }
            })
        } else if target.is_absolute() {
            target.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(target))
                .unwrap_or_else(|_| target.to_path_buf())
        }
    }

    /// Returns the cache file path for a given target file.
    /// Pattern: `<temp_dir>/.ie_cache_<hash>`
    pub fn path(target: &Path) -> PathBuf {
        let canonical = Self::canonical_path(target);
        let mut hasher = DefaultHasher::new();
        canonical.to_string_lossy().hash(&mut hasher);
        let hash = format!("{:016x}", hasher.finish());
        Self::temp_dir().join(format!(".ie_cache_{}", hash))
    }

    /// Checks if a temporary cache file exists for the target file.
    pub fn exists(target: &Path) -> bool {
        Self::path(target).exists()
    }

    /// Reads the cached content using `File::read` (mirroring grp's file reading pattern).
    pub fn read(target: &Path) -> Result<String, Error> {
        let cache_path = Self::path(target);
        File::read(&cache_path)
    }

    /// Saves content to the temporary cache using `File::write`.
    pub fn save(target: &Path, content: &str) -> Result<PathBuf, Error> {
        let cache_path = Self::path(target);
        File::write(&cache_path, content)?;
        Ok(cache_path)
    }

    /// Removes the temporary cache file if it exists.
    pub fn remove(target: &Path) -> Result<bool, Error> {
        let cache_path = Self::path(target);
        if cache_path.exists() {
            fs::remove_file(&cache_path).map_err(|e| {
                mynox::error!(
                    "fs::cache_delete",
                    "Failed to remove cache file",
                    format!("Could not remove cached file `{}`: {}", cache_path.display(), e)
                )
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_lifecycle() {
        let test_file = std::env::temp_dir().join(format!("ie_target_test_{}.txt", std::process::id()));
        let _ = fs::write(&test_file, "original text");

        // Cache initially does not exist
        assert!(!Cache::exists(&test_file));

        // Save to cache
        let cached_content = "edited text that failed to save";
        let save_res = Cache::save(&test_file, cached_content);
        assert!(save_res.is_ok());
        assert!(Cache::exists(&test_file));

        // Read from cache
        let read_res = Cache::read(&test_file);
        assert!(read_res.is_ok());
        assert_eq!(read_res.unwrap(), cached_content);

        // Remove from cache
        let rm_res = Cache::remove(&test_file);
        assert!(rm_res.is_ok());
        assert!(rm_res.unwrap());
        assert!(!Cache::exists(&test_file));

        // Cleanup
        let _ = fs::remove_file(&test_file);
    }
}
