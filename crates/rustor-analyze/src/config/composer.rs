//! Composer.json parsing for PSR-4 autoload support

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a parsed composer.json file
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ComposerJson {
    #[serde(default)]
    pub autoload: AutoloadSection,

    #[serde(default, rename = "autoload-dev")]
    pub autoload_dev: AutoloadSection,
}

/// Autoload configuration section
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AutoloadSection {
    #[serde(default, rename = "psr-4")]
    pub psr4: HashMap<String, Psr4Paths>,
}

/// PSR-4 paths can be a single string or an array of strings
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Psr4Paths {
    Single(String),
    Multiple(Vec<String>),
}

impl Psr4Paths {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Psr4Paths::Single(s) => vec![s.clone()],
            Psr4Paths::Multiple(v) => v.clone(),
        }
    }
}

/// A resolved PSR-4 mapping
#[derive(Debug, Clone)]
pub struct Psr4Mapping {
    pub namespace_prefix: String,
    pub directories: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum ComposerError {
    #[error("Failed to read composer.json: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse composer.json: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl ComposerJson {
    pub fn load(path: &Path) -> Result<Self, ComposerError> {
        let content = std::fs::read_to_string(path)?;
        let composer: ComposerJson = serde_json::from_str(&content)?;
        Ok(composer)
    }

    pub fn find_in_directory(dir: &Path) -> Option<PathBuf> {
        let mut current = dir.to_path_buf();
        loop {
            // Check common locations for composer.json
            // Prefer libs/composer.json which often has the main autoload in monorepos
            let paths_to_check = [
                current.join("libs/composer.json"),
                current.join("composer.json"),
            ];

            // First pass: find a composer.json with autoload
            for composer_path in &paths_to_check {
                if composer_path.exists() {
                    if let Ok(c) = Self::load(composer_path) {
                        if c.has_autoload() {
                            return Some(composer_path.clone());
                        }
                    }
                }
            }

            // Second pass: fallback to any existing composer.json
            for composer_path in &paths_to_check {
                if composer_path.exists() {
                    return Some(composer_path.clone());
                }
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    pub fn get_psr4_mappings(&self, base_dir: &Path, include_dev: bool) -> Vec<Psr4Mapping> {
        let mut mappings = Vec::new();

        for (namespace, paths) in &self.autoload.psr4 {
            let directories: Vec<PathBuf> = paths
                .to_vec()
                .into_iter()
                .map(|p| base_dir.join(&p))
                .collect();
            mappings.push(Psr4Mapping {
                namespace_prefix: namespace.clone(),
                directories,
            });
        }

        if include_dev {
            for (namespace, paths) in &self.autoload_dev.psr4 {
                let directories: Vec<PathBuf> = paths
                    .to_vec()
                    .into_iter()
                    .map(|p| base_dir.join(&p))
                    .collect();
                mappings.push(Psr4Mapping {
                    namespace_prefix: namespace.clone(),
                    directories,
                });
            }
        }

        mappings
    }

    pub fn has_autoload(&self) -> bool {
        !self.autoload.psr4.is_empty()
    }
}
