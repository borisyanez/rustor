//! Cache for autoload symbol tables

use crate::symbols::SymbolTable;
use crate::symbols::class_info::{ClassInfo, ClassMethodInfo, ClassKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// Bump version to 2 to invalidate old caches (now includes methods and traits)
const CACHE_VERSION: u32 = 2;
const CACHE_DIR: &str = ".rustor-cache";
const CACHE_FILE: &str = "vendor-symbols.json";

/// Cached class information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedClass {
    pub short_name: String,
    pub full_name: String,
    pub kind: u8,  // 0=Class, 1=Interface, 2=Trait, 3=Enum
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub traits: Vec<String>,
    pub methods: Vec<String>,  // Just method names for now
}

impl CachedClass {
    fn from_class_info(info: &ClassInfo) -> Self {
        Self {
            short_name: info.name.clone(),
            full_name: info.full_name.clone(),
            kind: match info.kind {
                ClassKind::Class => 0,
                ClassKind::Interface => 1,
                ClassKind::Trait => 2,
                ClassKind::Enum => 3,
            },
            parent: info.parent.clone(),
            interfaces: info.interfaces.clone(),
            traits: info.traits.clone(),
            methods: info.methods.keys().cloned().collect(),
        }
    }

    fn to_class_info(&self) -> ClassInfo {
        let mut info = ClassInfo::new(&self.short_name, &self.full_name);
        info.kind = match self.kind {
            0 => ClassKind::Class,
            1 => ClassKind::Interface,
            2 => ClassKind::Trait,
            3 => ClassKind::Enum,
            _ => ClassKind::Class,
        };
        info.parent = self.parent.clone();
        info.interfaces = self.interfaces.clone();
        info.traits = self.traits.clone();
        // Add methods (just names, no parameter info in cache)
        for method_name in &self.methods {
            info.add_method(ClassMethodInfo::new(method_name));
        }
        info
    }
}

/// Cached symbol information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedSymbols {
    version: u32,
    vendor_mtime: u64,
    classmap_mtime: u64,
    /// Map of class FQN -> cached class info
    classes: HashMap<String, CachedClass>,
}

impl CachedSymbols {
    /// Create a new cache from a symbol table
    pub fn from_symbol_table(table: &SymbolTable, vendor_mtime: u64, classmap_mtime: u64) -> Self {
        let mut classes = HashMap::new();
        for class in table.all_class_infos() {
            classes.insert(class.full_name.clone(), CachedClass::from_class_info(class));
        }
        Self {
            version: CACHE_VERSION,
            vendor_mtime,
            classmap_mtime,
            classes,
        }
    }

    /// Convert back to a symbol table
    pub fn to_symbol_table(&self) -> SymbolTable {
        let mut table = SymbolTable::new();
        for (_, cached) in &self.classes {
            table.register_class(cached.to_class_info());
        }
        table
    }
}

/// Autoload cache manager
pub struct AutoloadCache {
    cache_dir: PathBuf,
}

impl AutoloadCache {
    /// Create cache manager for a project directory
    pub fn for_project(project_dir: &Path) -> Self {
        Self {
            cache_dir: project_dir.join(CACHE_DIR),
        }
    }

    /// Try to load cached symbols
    pub fn load(&self, vendor_dir: &Path, classmap_path: &Path) -> Option<SymbolTable> {
        let cache_file = self.cache_dir.join(CACHE_FILE);
        if !cache_file.exists() {
            return None;
        }

        // Read cache
        let content = fs::read_to_string(&cache_file).ok()?;
        let cached: CachedSymbols = serde_json::from_str(&content).ok()?;

        // Check version
        if cached.version != CACHE_VERSION {
            return None;
        }

        // Check if vendor dir or classmap changed
        let vendor_mtime = get_mtime(vendor_dir).unwrap_or(0);
        let classmap_mtime = get_mtime(classmap_path).unwrap_or(0);

        if cached.vendor_mtime != vendor_mtime || cached.classmap_mtime != classmap_mtime {
            return None;
        }

        Some(cached.to_symbol_table())
    }

    /// Save symbol table to cache
    pub fn save(&self, table: &SymbolTable, vendor_dir: &Path, classmap_path: &Path) -> std::io::Result<()> {
        let vendor_mtime = get_mtime(vendor_dir).unwrap_or(0);
        let classmap_mtime = get_mtime(classmap_path).unwrap_or(0);

        let cached = CachedSymbols::from_symbol_table(table, vendor_mtime, classmap_mtime);
        let content = serde_json::to_string_pretty(&cached)?;

        fs::create_dir_all(&self.cache_dir)?;
        fs::write(self.cache_dir.join(CACHE_FILE), content)?;

        Ok(())
    }
}

fn get_mtime(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}
