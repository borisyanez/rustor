//! File caching for rustor to skip unchanged files
//!
//! Uses xxHash for fast file hashing and stores cache in `.rustor-cache` file.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

/// Cache file name
const CACHE_FILE: &str = ".rustor-cache";

/// Cache version - increment when format changes
const CACHE_VERSION: u32 = 1;

/// Entry for a single cached file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// xxHash of file contents
    pub content_hash: u64,
    /// Hash of the enabled rules set
    pub rules_hash: u64,
    /// Whether the file had any suggested edits
    pub has_edits: bool,
    /// Number of edits found
    pub edit_count: usize,
}

/// Cache structure stored on disk
#[derive(Debug, Serialize, Deserialize)]
pub struct Cache {
    /// Cache format version
    pub version: u32,
    /// Cached entries by file path (relative to cache file location)
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }
}

impl Cache {
    /// Load cache from the default location in the given directory
    pub fn load(dir: &Path) -> Result<Self> {
        let cache_path = dir.join(CACHE_FILE);
        Self::load_from(&cache_path)
    }

    /// Load cache from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read cache file: {}", path.display()))?;

        let cache: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse cache file: {}", path.display()))?;

        // Check version compatibility
        if cache.version != CACHE_VERSION {
            // Version mismatch - return empty cache
            return Ok(Self::default());
        }

        Ok(cache)
    }

    /// Save cache to the default location in the given directory
    pub fn save(&self, dir: &Path) -> Result<()> {
        let cache_path = dir.join(CACHE_FILE);
        self.save_to(&cache_path)
    }

    /// Save cache to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize cache")?;

        fs::write(path, contents)
            .with_context(|| format!("Failed to write cache file: {}", path.display()))?;

        Ok(())
    }

    /// Check if a file is valid in the cache (unchanged since last run)
    #[allow(dead_code)]
    pub fn is_valid(&self, path: &Path, current_hash: u64, rules_hash: u64) -> bool {
        if let Some(entry) = self.entries.get(path) {
            entry.content_hash == current_hash && entry.rules_hash == rules_hash
        } else {
            false
        }
    }

    /// Get cached result for a file if valid
    pub fn get_if_valid(&self, path: &Path, current_hash: u64, rules_hash: u64) -> Option<&CacheEntry> {
        self.entries.get(path).filter(|entry| {
            entry.content_hash == current_hash && entry.rules_hash == rules_hash
        })
    }

    /// Update cache entry for a file
    pub fn update(&mut self, path: PathBuf, content_hash: u64, rules_hash: u64, has_edits: bool, edit_count: usize) {
        self.entries.insert(path, CacheEntry {
            content_hash,
            rules_hash,
            has_edits,
            edit_count,
        });
    }

    /// Remove a file from the cache
    #[allow(dead_code)]
    pub fn invalidate(&mut self, path: &Path) {
        self.entries.remove(path);
    }

    /// Clear all entries
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove entries for files that no longer exist
    #[allow(dead_code)]
    pub fn prune(&mut self, base_dir: &Path) {
        self.entries.retain(|path, _| {
            let full_path = if path.is_absolute() {
                path.clone()
            } else {
                base_dir.join(path)
            };
            full_path.exists()
        });
    }

    /// Get number of cached entries
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Hash a file's contents using xxHash (very fast)
pub fn hash_file(path: &Path) -> Result<u64> {
    let contents = fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    Ok(xxh3_64(&contents))
}

/// Hash a set of rule names to detect rule configuration changes
pub fn hash_rules(rules: &HashSet<String>) -> u64 {
    let mut sorted_rules: Vec<_> = rules.iter().map(|s| s.as_str()).collect();
    sorted_rules.sort();
    let combined = sorted_rules.join(",");
    xxh3_64(combined.as_bytes())
}

/// Delete the cache file in the given directory
pub fn clear_cache(dir: &Path) -> Result<()> {
    let cache_path = dir.join(CACHE_FILE);
    if cache_path.exists() {
        fs::remove_file(&cache_path)
            .with_context(|| format!("Failed to delete cache file: {}", cache_path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_save_load() {
        let temp = TempDir::new().unwrap();
        let mut cache = Cache::default();

        cache.update(
            PathBuf::from("test.php"),
            12345,
            67890,
            true,
            3,
        );

        cache.save(temp.path()).unwrap();

        let loaded = Cache::load(temp.path()).unwrap();
        assert_eq!(loaded.entries.len(), 1);

        let entry = loaded.entries.get(&PathBuf::from("test.php")).unwrap();
        assert_eq!(entry.content_hash, 12345);
        assert_eq!(entry.rules_hash, 67890);
        assert!(entry.has_edits);
        assert_eq!(entry.edit_count, 3);
    }

    #[test]
    fn test_cache_is_valid() {
        let mut cache = Cache::default();
        let path = PathBuf::from("test.php");

        cache.update(path.clone(), 12345, 67890, false, 0);

        // Same hashes - valid
        assert!(cache.is_valid(&path, 12345, 67890));

        // Different content hash - invalid
        assert!(!cache.is_valid(&path, 99999, 67890));

        // Different rules hash - invalid
        assert!(!cache.is_valid(&path, 12345, 99999));

        // Non-existent file - invalid
        assert!(!cache.is_valid(&PathBuf::from("other.php"), 12345, 67890));
    }

    #[test]
    fn test_hash_rules() {
        let mut rules1 = HashSet::new();
        rules1.insert("array_push".to_string());
        rules1.insert("is_null".to_string());

        let mut rules2 = HashSet::new();
        rules2.insert("is_null".to_string());
        rules2.insert("array_push".to_string());

        // Order shouldn't matter
        assert_eq!(hash_rules(&rules1), hash_rules(&rules2));

        // Different rules should have different hash
        let mut rules3 = HashSet::new();
        rules3.insert("sizeof".to_string());
        assert_ne!(hash_rules(&rules1), hash_rules(&rules3));
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = Cache::default();
        let path = PathBuf::from("test.php");

        cache.update(path.clone(), 12345, 67890, false, 0);
        assert!(cache.entries.contains_key(&path));

        cache.invalidate(&path);
        assert!(!cache.entries.contains_key(&path));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = Cache::default();
        cache.update(PathBuf::from("a.php"), 1, 1, false, 0);
        cache.update(PathBuf::from("b.php"), 2, 2, false, 0);

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_load_nonexistent() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::load(temp.path()).unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_version_mismatch() {
        let temp = TempDir::new().unwrap();
        let cache_path = temp.path().join(CACHE_FILE);

        // Write cache with different version
        let old_cache = serde_json::json!({
            "version": 999,
            "entries": {}
        });
        fs::write(&cache_path, old_cache.to_string()).unwrap();

        // Should return empty cache due to version mismatch
        let cache = Cache::load(temp.path()).unwrap();
        assert!(cache.is_empty());
        assert_eq!(cache.version, CACHE_VERSION);
    }
}
