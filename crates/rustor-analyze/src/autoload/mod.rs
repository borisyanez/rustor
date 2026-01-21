//! Composer autoload support for scanning PHP files

pub mod cache;
pub mod include_scanner;

use crate::config::composer::{ComposerJson, Psr4Mapping};
use crate::resolver::symbol_collector::{CollectedSymbols, SymbolCollector};
use crate::symbols::{ClassInfo, SymbolTable};
use mago_database::file::FileId;
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scanner for collecting symbols from Composer autoload paths
pub struct AutoloadScanner {
    mappings: Vec<Psr4Mapping>,
    #[allow(dead_code)]
    base_dir: PathBuf,
}

impl AutoloadScanner {
    pub fn from_composer(composer: &ComposerJson, base_dir: &Path, include_dev: bool) -> Self {
        let mappings = composer.get_psr4_mappings(base_dir, include_dev);
        Self {
            mappings,
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn discover_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for mapping in &self.mappings {
            for dir in &mapping.directories {
                if !dir.exists() {
                    continue;
                }

                for entry in WalkDir::new(dir)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_file() && path.extension().map(|e| e == "php").unwrap_or(false) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        files
    }

    pub fn build_symbol_table(&self) -> SymbolTable {
        let files = self.discover_files();

        let collected: Vec<CollectedSymbols> = files
            .par_iter()
            .filter_map(|file| {
                let source = fs::read_to_string(file).ok()?;
                let arena = bumpalo::Bump::new();
                let file_id = FileId::new(file.to_string_lossy().as_ref());
                let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                let collector = SymbolCollector::new(&source, file);
                Some(collector.collect(&program))
            })
            .collect();

        SymbolCollector::build_symbol_table_from_symbols(collected)
    }

    pub fn stats(&self) -> AutoloadStats {
        let files = self.discover_files();
        AutoloadStats {
            mapping_count: self.mappings.len(),
            directory_count: self.mappings.iter().map(|m| m.directories.len()).sum(),
            file_count: files.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoloadStats {
    pub mapping_count: usize,
    pub directory_count: usize,
    pub file_count: usize,
}

/// Scanner for Composer's autoload_classmap.php
pub struct ClassmapScanner {
    classmap_path: PathBuf,
}

impl ClassmapScanner {
    /// Find autoload_classmap.php by searching from a directory upward
    /// Prefers the classmap with more content if multiple exist
    pub fn find_from_directory(dir: &Path) -> Option<Self> {
        let mut current = dir.to_path_buf();
        loop {
            // Check common vendor locations
            let paths_to_check = [
                current.join("vendor/composer/autoload_classmap.php"),
                current.join("libs/vendor/composer/autoload_classmap.php"),
            ];

            // Find all existing classmaps and pick the largest one
            let mut best: Option<(PathBuf, u64)> = None;
            for classmap_path in paths_to_check {
                if classmap_path.exists() {
                    let size = fs::metadata(&classmap_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if best.is_none() || size > best.as_ref().unwrap().1 {
                        best = Some((classmap_path, size));
                    }
                }
            }

            if let Some((classmap_path, _)) = best {
                return Some(Self { classmap_path });
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Parse the classmap file and return class names
    pub fn parse_classes(&self) -> Vec<String> {
        // Read as bytes and convert to string lossily to handle non-UTF-8
        let bytes = match fs::read(&self.classmap_path) {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };
        let content = String::from_utf8_lossy(&bytes);

        // Match patterns like: 'ClassName' => or "ClassName" =>
        // The class name is the key in the array
        // Need to handle escaped backslashes in class names like 'AWS\\CRT\\Auth\\AwsCredentials'
        let re = Regex::new(r"'([^']+)'\s*=>").unwrap();

        let classes: Vec<String> = re.captures_iter(&content)
            .filter_map(|cap| {
                cap.get(1).map(|m| {
                    // Convert PHP escaped backslashes (\\) to single backslash (\)
                    m.as_str().replace("\\\\", "\\")
                })
            })
            .collect();

        classes
    }

    /// Build a symbol table from the classmap
    pub fn build_symbol_table(&self) -> SymbolTable {
        let classes = self.parse_classes();
        let mut table = SymbolTable::new();

        for class_name in classes {
            // Extract short name from FQN
            let short_name = class_name
                .rsplit('\\')
                .next()
                .unwrap_or(&class_name)
                .to_string();

            let info = ClassInfo::new(&short_name, &class_name);
            table.register_class(info);
        }

        table
    }

    /// Get statistics
    pub fn stats(&self) -> ClassmapStats {
        let classes = self.parse_classes();
        ClassmapStats {
            class_count: classes.len(),
            classmap_path: self.classmap_path.clone(),
        }
    }

    /// Get the classmap path for cache validation
    pub fn classmap_path(&self) -> PathBuf {
        self.classmap_path.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ClassmapStats {
    pub class_count: usize,
    pub classmap_path: PathBuf,
}

/// Scanner for Composer's autoload_psr4.php (vendor PSR-4 mappings)
pub struct VendorPsr4Scanner {
    psr4_path: PathBuf,
    vendor_dir: PathBuf,
}

impl VendorPsr4Scanner {
    /// Find autoload_psr4.php by searching from a directory upward
    /// Prefers the PSR-4 file with more content if multiple exist
    pub fn find_from_directory(dir: &Path) -> Option<Self> {
        let mut current = dir.to_path_buf();
        loop {
            // Check common vendor locations
            let paths_to_check = [
                (
                    current.join("vendor/composer/autoload_psr4.php"),
                    current.join("vendor"),
                ),
                (
                    current.join("libs/vendor/composer/autoload_psr4.php"),
                    current.join("libs/vendor"),
                ),
            ];

            // Find all existing PSR-4 files and pick the largest one
            let mut best: Option<(PathBuf, PathBuf, u64)> = None;
            for (psr4_path, vendor_dir) in paths_to_check {
                if psr4_path.exists() {
                    let size = fs::metadata(&psr4_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if best.is_none() || size > best.as_ref().unwrap().2 {
                        best = Some((psr4_path, vendor_dir, size));
                    }
                }
            }

            if let Some((psr4_path, vendor_dir, _)) = best {
                return Some(Self { psr4_path, vendor_dir });
            }

            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Parse the PSR-4 file and return namespace to directory mappings
    pub fn parse_mappings(&self) -> Vec<(String, PathBuf)> {
        let bytes = match fs::read(&self.psr4_path) {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };
        let content = String::from_utf8_lossy(&bytes);

        let mut mappings = Vec::new();

        // Match patterns like:
        // 'Psr\\Log\\' => array($vendorDir . '/psr/log/src'),
        // 'Psr\\Http\\Message\\' => array($vendorDir . '/psr/http-factory/src', $vendorDir . '/psr/http-message/src'),
        // First, extract namespace => array(...) pairs
        let ns_re = Regex::new(r"'([^']+)'\s*=>\s*array\(([^)]+)\)").unwrap();

        for cap in ns_re.captures_iter(&content) {
            if let (Some(ns), Some(array_content)) = (cap.get(1), cap.get(2)) {
                let namespace = ns.as_str().replace("\\\\", "\\");
                let array_str = array_content.as_str();

                // Extract all $vendorDir . '/path' entries from the array
                let path_re = Regex::new(r"\$vendorDir\s*\.\s*'([^']+)'").unwrap();
                for path_cap in path_re.captures_iter(array_str) {
                    if let Some(path) = path_cap.get(1) {
                        let rel_path = path.as_str().trim_start_matches('/');
                        let full_path = self.vendor_dir.join(rel_path);
                        if full_path.exists() {
                            mappings.push((namespace.clone(), full_path));
                        }
                    }
                }
            }
        }

        mappings
    }

    /// Build a symbol table by scanning PSR-4 directories
    pub fn build_symbol_table(&self) -> SymbolTable {
        let mappings = self.parse_mappings();
        let mut table = SymbolTable::new();

        for (namespace, dir) in mappings {
            // Walk the directory and collect PHP files
            for entry in WalkDir::new(&dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "php").unwrap_or(false) {
                    // Parse and collect symbols
                    if let Ok(source) = fs::read_to_string(path) {
                        let arena = bumpalo::Bump::new();
                        let file_id = FileId::new(path.to_string_lossy().as_ref());
                        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                        let collector = SymbolCollector::new(&source, path);
                        let symbols = collector.collect(&program);

                        // Register collected classes
                        for class in symbols.classes {
                            table.register_class(class);
                        }
                    }
                }
            }
        }

        table
    }

    /// Get statistics
    pub fn stats(&self) -> VendorPsr4Stats {
        let mappings = self.parse_mappings();
        VendorPsr4Stats {
            mapping_count: mappings.len(),
            psr4_path: self.psr4_path.clone(),
        }
    }

    /// Get the vendor directory path for cache validation
    pub fn vendor_dir(&self) -> PathBuf {
        self.vendor_dir.clone()
    }
}

#[derive(Debug, Clone)]
pub struct VendorPsr4Stats {
    pub mapping_count: usize,
    pub psr4_path: PathBuf,
}
