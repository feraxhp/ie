use std::fs;
use std::path::{Path, PathBuf};
use mynox::Error;

/// Detector for file and directory write permissions.
/// Prevents panics or unexpected crashes when attempting to save a file in a location without write permissions.
pub fn check_write_permission(path: &Path) -> Result<(), Error> {
    if path.exists() {
        // Check if the current process has actual write access to the file.
        // We probe opening with write access rather than solely rejecting on metadata.permissions().readonly(),
        // because root/CAP_DAC_OVERRIDE can write to 0440 files (such as /etc/sudoers during visudo),
        // whereas metadata.readonly() is true for 0440 and would falsely block root writes.
        match fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                let metadata = fs::metadata(path).ok();
                if metadata.map(|m| m.permissions().readonly()).unwrap_or(false) {
                    Err(mynox::error!(
                        "fs::permission_denied",
                        "Permission denied",
                        format!("File `{}` is **read-only**", path.display())
                        =>
                        format!("Run `chmod +w {}` to grant write permissions", path.display())
                    ))
                } else {
                    Err(mynox::error!(
                        "fs::permission_denied",
                        "Permission denied",
                        format!("No **write permissions** for file `{}`", path.display())
                        =>
                        "Check your user permissions or run with elevated privileges"
                    ))
                }
            }
            Err(e) => {
                Err(mynox::error!(
                    "fs::io_error",
                    "Failed to open file for writing",
                    format!("Access error for `{}`: {}", path.display(), e)
                ))
            }
        }
    } else {
        // The file does not exist yet. Check if the nearest existing ancestor directory allows creating files.
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };

        let mut current: &Path = parent;
        while !current.exists() {
            match current.parent() {
                Some(p) if !p.as_os_str().is_empty() => current = p,
                _ => current = Path::new("."),
            }
        }

        let metadata = fs::metadata(current).map_err(|e| {
            mynox::error!(
                "fs::permission_denied",
                "Cannot access directory",
                format!("Failed to read metadata for directory `{}`: {}", current.display(), e)
            )
        })?;

        if metadata.permissions().readonly() {
            return Err(mynox::error!(
                "fs::permission_denied",
                "Permission denied",
                format!("Directory `{}` is **read-only**", current.display())
            ));
        }

        // Probe whether creating new files is permitted in this directory
        let probe_name = format!(
            ".ie_probe_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let probe_path = current.join(probe_name);

        match fs::OpenOptions::new().write(true).create_new(true).open(&probe_path) {
            Ok(file) => {
                drop(file);
                let _ = fs::remove_file(&probe_path);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(mynox::error!(
                    "fs::permission_denied",
                    "Permission denied",
                    format!("Cannot create files in directory `{}`: **permission denied**", current.display())
                    =>
                    "Check directory permissions or run with elevated privileges"
                ))
            }
            Err(_) => {
                // Allow proceeding if the probe failure is not a permission denial
                Ok(())
            }
        }
    }
}

/// Derives the backup file path by appending `.bak` to the given path.
pub fn backup_path(path: &Path) -> PathBuf {
    let mut bak = path.as_os_str().to_os_string();
    bak.push(".bak");
    PathBuf::from(bak)
}

/// Creates a backup copy (<path>.bak) of the file at `path` before modification.
/// If the file does not exist on disk, no backup is created and returns `Ok(None)`.
/// Returns `Ok(Some(backup_path))` if a backup was successfully created.
pub fn create_backup(path: &Path) -> Result<Option<PathBuf>, Error> {
    if !path.exists() {
        return Ok(None);
    }

    let bak_path = backup_path(path);

    // Validate write permissions before attempting to write/overwrite the backup
    check_write_permission(&bak_path)?;

    // Ensure parent directory exists before writing backup
    if let Some(parent) = bak_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                mynox::error!(
                    "fs::create_dir",
                    "Failed to create parent directory",
                    format!("Could not create directory `{}`: {}", parent.display(), e)
                )
            })?;
        }
    }

    fs::copy(path, &bak_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            mynox::error!(
                "fs::permission_denied",
                "Permission denied",
                format!("No **write permissions** to create backup `{}`", bak_path.display())
                =>
                "Verify file permissions or run with elevated privileges"
            )
        } else {
            mynox::error!(
                "fs::backup_error",
                "Failed to create backup file",
                format!("Could not create backup `{}`: {}", bak_path.display(), e)
                =>
                "Check available disk space and **write permissions**"
            )
        }
    })?;

    Ok(Some(bak_path))
}

/// Saves the given content to disk after validating write permissions.
/// If the file exists and its content is identical to what is being saved,
/// writing to disk is skipped to preserve the file's modification timestamp (mtime).
/// Tools like `crontab -e` and `visudo` inspect mtime to notify if no changes were made.
/// Existing inodes and POSIX permissions are preserved when writing.
pub fn save_file(content: &str, path: &Path) -> Result<(), Error> {
    // If the file already exists and its content is identical, do not rewrite it to preserve mtime
    if path.exists() {
        if let Ok(existing_bytes) = fs::read(path) {
            if existing_bytes == content.as_bytes() {
                return Ok(());
            }
        }
    }

    check_write_permission(path)?;

    // Ensure parent directory exists before writing
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                mynox::error!(
                    "fs::create_dir",
                    "Failed to create parent directory",
                    format!("Could not create directory `{}`: {}", parent.display(), e)
                )
            })?;
        }
    }

    fs::write(path, content).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            mynox::error!(
                "fs::permission_denied",
                "Permission denied",
                format!("No **write permissions** for file `{}`", path.display())
                =>
                "Verify file permissions or run with elevated privileges"
            )
        } else {
            mynox::error!(
                "fs::write_error",
                "Failed to save file",
                format!("Could not write to `{}`: {}", path.display(), e)
                =>
                "Check available disk space and **write permissions**"
            )
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_path() {
        assert_eq!(
            backup_path(Path::new("config.toml")),
            PathBuf::from("config.toml.bak")
        );
        assert_eq!(
            backup_path(Path::new("/etc/fstab")),
            PathBuf::from("/etc/fstab.bak")
        );
        assert_eq!(
            backup_path(Path::new("file")),
            PathBuf::from("file.bak")
        );
    }

    #[test]
    fn test_create_backup_existing_file() {
        let test_file = std::env::temp_dir().join(format!("ie_backup_test_{}.txt", std::process::id()));
        let bak_file = backup_path(&test_file);
        let _ = fs::write(&test_file, "original content");

        let res = create_backup(&test_file);
        assert!(res.is_ok());
        let res_path = res.unwrap();
        assert_eq!(res_path, Some(bak_file.clone()));
        assert!(bak_file.exists());
        assert_eq!(fs::read_to_string(&bak_file).unwrap(), "original content");

        // Clean up
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_file(&bak_file);
    }

    #[test]
    fn test_create_backup_nonexistent_file() {
        let test_file = std::env::temp_dir().join(format!("ie_nonexistent_backup_{}.txt", std::process::id()));
        let bak_file = backup_path(&test_file);

        let res = create_backup(&test_file);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
        assert!(!bak_file.exists());
    }

    #[test]
    fn test_create_backup_readonly_file() {
        let test_file = std::env::temp_dir().join(format!("ie_ro_bak_src_{}.txt", std::process::id()));
        let bak_file = backup_path(&test_file);
        let _ = fs::write(&test_file, "original content");
        let _ = fs::write(&bak_file, "old backup content");

        // Mark backup file as read-only
        let mut perms = fs::metadata(&bak_file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&bak_file, perms).unwrap();

        let res = create_backup(&test_file);
        assert!(res.is_err());
        assert!(res.unwrap_err().is_category("fs::permission_denied"));

        // Clean up
        let mut perms = fs::metadata(&bak_file).unwrap().permissions();
        perms.set_readonly(false);
        let _ = fs::set_permissions(&bak_file, perms);
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_file(&bak_file);
    }

    #[test]
    fn test_writable_file() {
        let test_file = std::env::temp_dir().join(format!("ie_writable_test_{}.txt", std::process::id()));
        let _ = fs::write(&test_file, "initial content");

        let res = check_write_permission(&test_file);
        assert!(res.is_ok());

        let save_res = save_file("updated content", &test_file);
        assert!(save_res.is_ok());
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "updated content");

        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_readonly_file_detected_without_panic() {
        let test_file = std::env::temp_dir().join(format!("ie_readonly_test_{}.txt", std::process::id()));
        let _ = fs::write(&test_file, "readonly content");

        // Mark file as read-only
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&test_file, perms).unwrap();

        // Detector must return error without panicking
        let check_res = check_write_permission(&test_file);
        assert!(check_res.is_err());
        let err = check_res.unwrap_err();
        assert!(err.is_category("fs::permission_denied"));

        // Save must also fail cleanly with fs::permission_denied
        let save_res = save_file("should fail", &test_file);
        assert!(save_res.is_err());
        let save_err = save_res.unwrap_err();
        assert!(save_err.is_category("fs::permission_denied"));

        // Restore write permissions to clean up
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_readonly(false);
        let _ = fs::set_permissions(&test_file, perms);
        let _ = fs::remove_file(&test_file);
    }

    #[test]
    #[cfg(unix)]
    fn test_save_file_preserves_inode_and_permissions() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let test_file = std::env::temp_dir().join(format!("ie_inode_test_{}.txt", std::process::id()));
        let _ = fs::write(&test_file, "initial content");

        // Set strict permissions (0600) like crontab and visudo
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&test_file, perms).unwrap();

        let initial_meta = fs::metadata(&test_file).unwrap();
        let initial_inode = initial_meta.ino();
        let initial_mode = initial_meta.permissions().mode() & 0o777;

        // Save updated content
        let save_res = save_file("updated content", &test_file);
        assert!(save_res.is_ok());

        let updated_meta = fs::metadata(&test_file).unwrap();
        assert_eq!(updated_meta.ino(), initial_inode, "Inode must be preserved across saves");
        assert_eq!(
            updated_meta.permissions().mode() & 0o777,
            initial_mode,
            "Permissions mode must be preserved across saves"
        );
        assert_eq!(fs::read_to_string(&test_file).unwrap(), "updated content");

        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_save_file_preserves_mtime_when_content_unchanged() {
        let test_file = std::env::temp_dir().join(format!("ie_mtime_test_{}.txt", std::process::id()));
        let _ = fs::write(&test_file, "same content");

        let initial_mtime = fs::metadata(&test_file).unwrap().modified().unwrap();

        // Brief sleep to ensure filesystem timestamp resolution would change if modified
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Save identical content: mtime must NOT be touched
        let save_res = save_file("same content", &test_file);
        assert!(save_res.is_ok());

        let after_same_mtime = fs::metadata(&test_file).unwrap().modified().unwrap();
        assert_eq!(
            after_same_mtime, initial_mtime,
            "mtime must be preserved when content is unchanged"
        );

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Save different content: mtime MUST be updated
        let save_res = save_file("different content", &test_file);
        assert!(save_res.is_ok());

        let after_diff_mtime = fs::metadata(&test_file).unwrap().modified().unwrap();
        assert!(
            after_diff_mtime > initial_mtime,
            "mtime must be updated when content is modified"
        );

        let _ = fs::remove_file(&test_file);
    }
}
