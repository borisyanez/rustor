//! Configuration parsing for PHP-CS-Fixer compatibility
//!
//! This module handles parsing `.php-cs-fixer.php` configuration files
//! and converting them into rustor's internal configuration format.

mod php_parser;
mod whitespace;
mod presets;

pub use php_parser::{PhpCsFixerConfig, FinderConfig, RuleConfig, parse_php_cs_fixer_config, ConfigValue as PhpConfigValue};
pub use whitespace::{WhitespaceConfig, IndentStyle, LineEnding};
pub use presets::{Preset, get_preset_rules, get_preset_options, PresetOptionValue};
