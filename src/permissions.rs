use std::fs;
use std::path::Path;
use mynox::Error;

/// Detector for file and directory write permissions.
/// Prevents panics or unexpected crashes when attempting to save a file in a location without write permissions.
pub fn check_write_permission(path: &Path) -> Result<(), Error> {
    if path.exists() {
        // 1. Check if the file has a read-only metadata flag
        let metadata = fs::metadata(path).map_err(|e| {
            mynox::error!(
                "fs::permission_denied",
                "Cannot access file metadata",
                format!("Failed to read metadata for `{}`: {}", path.display(), e)
            )
        })?;

        if metadata.permissions().readonly() {
            return Err(mynox::error!(
                "fs::permission_denied",
                "Permission denied",
                format!("File `{}` is **read-only**", path.display())
                =>
                format!("Run `chmod +w {}` to grant write permissions", path.display())
            ));
        }

        // 2. Check if the current process has actual write access to the file
        match fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(mynox::error!(
                    "fs::permission_denied",
                    "Permission denied",
                    format!("No **write permissions** for file `{}`", path.display())
                    =>
                    "Check your user permissions or run with elevated privileges"
                ))
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

/// Saves the given content to disk after validating write permissions.
pub fn save_file(content: &str, path: &Path) -> Result<(), Error> {
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
}
