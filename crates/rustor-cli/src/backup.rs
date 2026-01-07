//! Backup functionality for safe file modifications
//!
//! Creates timestamped backup directories before applying fixes,
//! allowing restoration if something goes wrong.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;

/// Backup manager for file modifications
pub struct BackupManager {
    /// Directory to store backups
    backup_dir: PathBuf,
    /// Timestamped session directory
    session_dir: Option<PathBuf>,
    /// Whether backups are enabled
    enabled: bool,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(backup_dir: PathBuf, enabled: bool) -> Self {
        Self {
            backup_dir,
            session_dir: None,
            enabled,
        }
    }

    /// Initialize a new backup session (creates timestamped directory)
    pub fn init_session(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let session_dir = self.backup_dir.join(&timestamp);

        fs::create_dir_all(&session_dir)
            .with_context(|| format!("Failed to create backup directory: {}", session_dir.display()))?;

        self.session_dir = Some(session_dir);
        Ok(())
    }

    /// Backup a file before modification
    pub fn backup_file(&self, path: &Path) -> Result<Option<PathBuf>> {
        if !self.enabled {
            return Ok(None);
        }

        let session_dir = match &self.session_dir {
            Some(dir) => dir,
            None => return Ok(None),
        };

        // Create relative path structure in backup
        let backup_path = if path.is_absolute() {
            // Strip leading / and use path components
            let relative: PathBuf = path.components()
                .skip(1) // Skip root
                .collect();
            session_dir.join(relative)
        } else {
            session_dir.join(path)
        };

        // Create parent directories
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create backup directory: {}", parent.display()))?;
        }

        // Copy file to backup location
        fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to backup file: {} -> {}", path.display(), backup_path.display()))?;

        Ok(Some(backup_path))
    }

    /// Restore a file from backup
    pub fn restore_file(&self, original_path: &Path, backup_path: &Path) -> Result<()> {
        fs::copy(backup_path, original_path)
            .with_context(|| format!("Failed to restore file: {} -> {}", backup_path.display(), original_path.display()))?;
        Ok(())
    }

    /// Get the session directory path (for reporting)
    pub fn session_path(&self) -> Option<&Path> {
        self.session_dir.as_deref()
    }

    /// Check if backups are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Verify a PHP file parses correctly
pub fn verify_php_file(path: &Path) -> Result<bool> {
    use bumpalo::Bump;
    use mago_database::file::FileId;

    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file for verification: {}", path.display()))?;

    let arena = Bump::new();
    let file_id = FileId::new(path.to_string_lossy().as_ref());
    let (_, parse_error) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

    Ok(parse_error.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_disabled() {
        let temp = TempDir::new().unwrap();
        let manager = BackupManager::new(temp.path().to_path_buf(), false);

        assert!(!manager.is_enabled());
        assert!(manager.session_path().is_none());
    }

    #[test]
    fn test_backup_session() {
        let temp = TempDir::new().unwrap();
        let mut manager = BackupManager::new(temp.path().to_path_buf(), true);

        manager.init_session().unwrap();

        assert!(manager.is_enabled());
        assert!(manager.session_path().is_some());
        assert!(manager.session_path().unwrap().exists());
    }

    #[test]
    fn test_backup_file() {
        let temp = TempDir::new().unwrap();
        let mut manager = BackupManager::new(temp.path().join("backups"), true);

        // Create a test file
        let test_file = temp.path().join("test.php");
        fs::write(&test_file, "<?php echo 'hello';").unwrap();

        manager.init_session().unwrap();
        let backup_path = manager.backup_file(&test_file).unwrap();

        assert!(backup_path.is_some());
        let backup = backup_path.unwrap();
        assert!(backup.exists());

        // Verify content matches
        let original = fs::read_to_string(&test_file).unwrap();
        let backed_up = fs::read_to_string(&backup).unwrap();
        assert_eq!(original, backed_up);
    }

    #[test]
    fn test_restore_file() {
        let temp = TempDir::new().unwrap();
        let mut manager = BackupManager::new(temp.path().join("backups"), true);

        // Create a test file
        let test_file = temp.path().join("test.php");
        fs::write(&test_file, "<?php echo 'original';").unwrap();

        manager.init_session().unwrap();
        let backup_path = manager.backup_file(&test_file).unwrap().unwrap();

        // Modify original
        fs::write(&test_file, "<?php echo 'modified';").unwrap();

        // Restore
        manager.restore_file(&test_file, &backup_path).unwrap();

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "<?php echo 'original';");
    }

    #[test]
    fn test_verify_valid_php() {
        let temp = TempDir::new().unwrap();
        let test_file = temp.path().join("valid.php");
        fs::write(&test_file, "<?php\necho 'hello';\n").unwrap();

        assert!(verify_php_file(&test_file).unwrap());
    }

    #[test]
    fn test_verify_invalid_php() {
        let temp = TempDir::new().unwrap();
        let test_file = temp.path().join("invalid.php");
        fs::write(&test_file, "<?php\necho 'hello\n").unwrap(); // Missing closing quote

        assert!(!verify_php_file(&test_file).unwrap());
    }
}
