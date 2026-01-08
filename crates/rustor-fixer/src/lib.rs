//! rustor-fixer: PHP-CS-Fixer compatible formatting rules for rustor
//!
//! This crate provides formatting fixers that are compatible with PHP-CS-Fixer,
//! allowing rustor to parse `.php-cs-fixer.php` configuration files and apply
//! the same formatting rules.
//!
//! # Features
//!
//! - Parse `.php-cs-fixer.php` configuration files directly
//! - PSR-12 preset support (~50 fixers)
//! - Same formatting output as PHP-CS-Fixer
//! - Priority-based execution order
//!
//! # Example
//!
//! ```ignore
//! use rustor_fixer::config::PhpCsFixerConfig;
//! use rustor_fixer::fixers::FixerRegistry;
//!
//! let config = PhpCsFixerConfig::from_file(".php-cs-fixer.php")?;
//! let registry = FixerRegistry::new();
//! let edits = registry.check_file(source, &config)?;
//! ```

pub mod config;
pub mod fixers;

pub use config::{PhpCsFixerConfig, WhitespaceConfig, IndentStyle, LineEnding};
pub use fixers::{Fixer, FixerRegistry, FixerConfig};
