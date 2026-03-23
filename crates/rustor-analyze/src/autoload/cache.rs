//! Cache for autoload symbol tables

use crate::symbols::SymbolTable;
use crate::symbols::class_info::{ClassInfo, ClassMethodInfo, ClassKind, MethodParameterInfo};
use crate::types::php_type::Visibility;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// Bump version to 4 to invalidate old caches (now stores full method parameter info)
const CACHE_VERSION: u32 = 4;
const CACHE_DIR: &str = ".rustor-cache";
const CACHE_FILE: &str = "vendor-symbols.json";

/// Cached method parameter information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedParameter {
    pub name: String,
    pub type_: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
}

/// Cached method information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedMethod {
    pub name: String,
    pub parameters: Vec<CachedParameter>,
    pub return_type: Option<String>,
    pub is_static: bool,
    pub visibility: u8,  // 0=public, 1=protected, 2=private
}

/// Cached class information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedClass {
    pub short_name: String,
    pub full_name: String,
    pub kind: u8,  // 0=Class, 1=Interface, 2=Trait, 3=Enum
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub traits: Vec<String>,
    pub methods: Vec<CachedMethod>,
    #[serde(default)]
    pub methods_fully_collected: bool,
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
            methods: info.methods.values().map(|m| {
                CachedMethod {
                    name: m.name.clone(),
                    parameters: m.parameters.iter().map(|p| {
                        CachedParameter {
                            name: p.name.clone(),
                            type_: p.type_.as_ref().map(|t| type_to_cache_string(t)),
                            is_optional: p.is_optional,
                            is_variadic: p.is_variadic,
                        }
                    }).collect(),
                    return_type: m.return_type.as_ref().map(|t| type_to_cache_string(t)),
                    is_static: m.is_static,
                    visibility: match m.visibility {
                        Visibility::Public => 0,
                        Visibility::Protected => 1,
                        Visibility::Private => 2,
                    },
                }
            }).collect(),
            methods_fully_collected: info.methods_fully_collected,
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
        info.methods_fully_collected = self.methods_fully_collected;
        for cached_method in &self.methods {
            let mut method = ClassMethodInfo::new(&cached_method.name);
            method.is_static = cached_method.is_static;
            method.visibility = match cached_method.visibility {
                1 => Visibility::Protected,
                2 => Visibility::Private,
                _ => Visibility::Public,
            };
            if let Some(ref rt) = cached_method.return_type {
                if let Some(ty) = crate::types::phpdoc::parse_type_string(rt) {
                    method.return_type = Some(ty);
                }
            }
            for cached_param in &cached_method.parameters {
                let mut param = MethodParameterInfo::new(&cached_param.name);
                param.is_optional = cached_param.is_optional;
                param.is_variadic = cached_param.is_variadic;
                if let Some(ref type_str) = cached_param.type_ {
                    if let Some(ty) = crate::types::phpdoc::parse_type_string(type_str) {
                        param.type_ = Some(ty);
                    }
                }
                method.parameters.push(param);
            }
            info.add_method(method);
        }
        info
    }
}

fn type_to_cache_string(ty: &crate::types::php_type::Type) -> String {
    use crate::types::php_type::Type;
    match ty {
        Type::Int | Type::ConstantInt(_) => "int".to_string(),
        Type::String | Type::ConstantString(_) => "string".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool | Type::ConstantBool(_) => "bool".to_string(),
        Type::Null => "null".to_string(),
        Type::Void => "void".to_string(),
        Type::Mixed => "mixed".to_string(),
        Type::Object { class_name: Some(name) } => name.clone(),
        Type::Nullable(inner) => format!("?{}", type_to_cache_string(inner)),
        Type::Union(types) => types.iter().map(|t| type_to_cache_string(t)).collect::<Vec<_>>().join("|"),
        Type::Array { .. } => "array".to_string(),
        Type::Callable | Type::Closure => "callable".to_string(),
        _ => "mixed".to_string(),
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
