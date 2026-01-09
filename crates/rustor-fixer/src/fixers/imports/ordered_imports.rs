//! Sort use imports alphabetically and by type

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, FixerOption, OptionType, ConfigValue, edit_with_rule};

/// Sorts use imports alphabetically, optionally grouping by type
pub struct OrderedImportsFixer;

impl Fixer for OrderedImportsFixer {
    fn name(&self) -> &'static str {
        "ordered_imports"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "ordered_imports"
    }

    fn description(&self) -> &'static str {
        "Sort use imports alphabetically"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn options(&self) -> Vec<FixerOption> {
        vec![
            FixerOption {
                name: "sort_algorithm",
                description: "Sorting algorithm: alpha, length, none",
                option_type: OptionType::Enum(vec!["alpha", "length", "none"]),
                default: Some(ConfigValue::String("alpha".to_string())),
            },
            FixerOption {
                name: "imports_order",
                description: "Order of import types: class, function, const",
                option_type: OptionType::StringArray,
                default: Some(ConfigValue::Array(vec![
                    "class".to_string(),
                    "function".to_string(),
                    "const".to_string(),
                ])),
            },
        ]
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Get sorting algorithm from config
        let sort_algo = config.options.get("sort_algorithm")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("alpha");

        // Find all use statement blocks
        let use_re = Regex::new(r"(?m)^([ \t]*)(use\s+(?:function\s+|const\s+)?[^;]+;)[ \t]*$").unwrap();

        // Collect consecutive use statements into blocks
        let mut blocks: Vec<UseBlock> = Vec::new();
        let mut current_block: Option<UseBlock> = None;
        let mut last_end = 0;

        for cap in use_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let use_stmt = cap.get(2).unwrap().as_str();

            // Check if this is consecutive with the previous use statement
            let is_consecutive = if let Some(ref block) = current_block {
                // Allow blank lines between use statements in same block
                let between = &source[block.end..full_match.start()];
                between.chars().all(|c| c.is_whitespace())
            } else {
                false
            };

            if is_consecutive {
                if let Some(ref mut block) = current_block {
                    block.statements.push(UseStatement::parse(use_stmt, indent));
                    block.end = full_match.end();
                }
            } else {
                // Save previous block
                if let Some(block) = current_block.take() {
                    if block.statements.len() > 1 {
                        blocks.push(block);
                    }
                }
                // Start new block
                current_block = Some(UseBlock {
                    start: full_match.start(),
                    end: full_match.end(),
                    indent: indent.to_string(),
                    statements: vec![UseStatement::parse(use_stmt, indent)],
                });
            }

            last_end = full_match.end();
        }

        // Don't forget the last block
        if let Some(block) = current_block {
            if block.statements.len() > 1 {
                blocks.push(block);
            }
        }

        // Sort each block and generate edits
        for block in blocks {
            let mut sorted = block.statements.clone();

            // Helper to get type ordering
            let type_order = |t: &UseType| match t {
                UseType::Class => 0,
                UseType::Function => 1,
                UseType::Const => 2,
            };

            match sort_algo {
                "alpha" => {
                    sorted.sort_by(|a, b| {
                        // Sort by type first (class < function < const), then alphabetically
                        let type_cmp = type_order(&a.use_type).cmp(&type_order(&b.use_type));
                        if type_cmp != std::cmp::Ordering::Equal {
                            return type_cmp;
                        }
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    });
                }
                "length" => {
                    sorted.sort_by(|a, b| {
                        a.full_statement.len().cmp(&b.full_statement.len())
                    });
                }
                "none" => {
                    // Group by type but preserve original order within each group
                    sorted.sort_by(|a, b| {
                        type_order(&a.use_type).cmp(&type_order(&b.use_type))
                    });
                }
                _ => {}
            }

            // Check if order changed
            let original_order: Vec<&str> = block.statements.iter()
                .map(|s| s.full_statement.as_str())
                .collect();
            let sorted_order: Vec<&str> = sorted.iter()
                .map(|s| s.full_statement.as_str())
                .collect();

            if original_order != sorted_order {
                // Generate new text with sorted imports
                let new_lines: Vec<String> = sorted
                    .iter()
                    .map(|stmt| format!("{}{}", stmt.indent, stmt.full_statement))
                    .collect();

                let new_text = new_lines.join(line_ending);

                edits.push(edit_with_rule(
                    block.start,
                    block.end,
                    new_text,
                    "Sort use imports alphabetically".to_string(),
                    "ordered_imports",
                ));
            }
        }

        edits
    }
}

#[derive(Clone)]
struct UseBlock {
    start: usize,
    end: usize,
    indent: String,
    statements: Vec<UseStatement>,
}

#[derive(Clone)]
struct UseStatement {
    indent: String,
    full_statement: String,
    name: String,
    use_type: UseType,
}

#[derive(Clone, PartialEq)]
enum UseType {
    Class,
    Function,
    Const,
}

impl UseStatement {
    fn parse(stmt: &str, indent: &str) -> Self {
        let use_type;
        let name;

        if stmt.starts_with("use function ") {
            use_type = UseType::Function;
            name = stmt.trim_start_matches("use function ")
                .trim_end_matches(';')
                .split(" as ")
                .next()
                .unwrap_or("")
                .to_string();
        } else if stmt.starts_with("use const ") {
            use_type = UseType::Const;
            name = stmt.trim_start_matches("use const ")
                .trim_end_matches(';')
                .split(" as ")
                .next()
                .unwrap_or("")
                .to_string();
        } else {
            use_type = UseType::Class;
            name = stmt.trim_start_matches("use ")
                .trim_end_matches(';')
                .split(" as ")
                .next()
                .unwrap_or("")
                .to_string();
        }

        UseStatement {
            indent: indent.to_string(),
            full_statement: stmt.to_string(),
            name,
            use_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        OrderedImportsFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_already_sorted() {
        let source = "<?php\n\nuse A\\B;\nuse C\\D;\nuse E\\F;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_unsorted() {
        let source = "<?php\n\nuse C\\D;\nuse A\\B;\nuse E\\F;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use A\\B;"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = "<?php\n\nuse Zoo\\Animal;\nuse apple\\Fruit;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        // apple should come before Zoo (case-insensitive)
        let lines: Vec<&str> = edits[0].replacement.lines().collect();
        assert!(lines[0].contains("apple"));
    }

    #[test]
    fn test_with_aliases() {
        let source = "<?php\n\nuse Zoo\\Animal as Z;\nuse Apple\\Fruit as A;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_grouped_by_type() {
        // Classes should come before functions, functions before consts
        let source = "<?php\n\nuse function strlen;\nuse App\\Model;\nuse const PHP_EOL;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        let lines: Vec<&str> = edits[0].replacement.lines().collect();
        assert!(lines[0].contains("use App\\Model"));
        assert!(lines[1].contains("use function strlen"));
        assert!(lines[2].contains("use const PHP_EOL"));
    }

    #[test]
    fn test_single_import_unchanged() {
        let source = "<?php\n\nuse App\\Model;\n\nclass Foo {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_non_consecutive_blocks() {
        let source = "<?php\n\nuse B\\B;\nuse A\\A;\n\nclass Foo {}\n\nuse D\\D;\nuse C\\C;\n";
        let edits = check(source);

        // Should have two separate edit blocks
        assert_eq!(edits.len(), 2);
    }
}
