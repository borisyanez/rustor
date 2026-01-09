//! Split grouped imports into separate statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, FixerOption, OptionType, ConfigValue, edit_with_rule};

/// Splits grouped use statements into individual statements
pub struct SingleImportPerStatementFixer;

impl Fixer for SingleImportPerStatementFixer {
    fn name(&self) -> &'static str {
        "single_import_per_statement"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_import_per_statement"
    }

    fn description(&self) -> &'static str {
        "Split grouped imports into separate statements"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn options(&self) -> Vec<FixerOption> {
        vec![
            FixerOption {
                name: "group_to_single_imports",
                description: "Whether to split grouped imports (use Foo\\{A, B}) into single imports. PSR-12 sets this to false.",
                option_type: OptionType::Bool,
                default: Some(ConfigValue::Bool(true)),
            },
        ]
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Check if we should split grouped imports
        let group_to_single = config.options.get("group_to_single_imports")
            .and_then(|v| match v {
                ConfigValue::Bool(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(true);

        // Match grouped imports: use Namespace\{Class1, Class2};
        // Only process if group_to_single_imports is true
        if group_to_single {
            let grouped_re = Regex::new(r"(?m)^([ \t]*)(use\s+(?:function\s+|const\s+)?)([\w\\]+)\s*\{([^}]+)\}\s*;").unwrap();

            for cap in grouped_re.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let indent = cap.get(1).unwrap().as_str();
                let use_prefix = cap.get(2).unwrap().as_str();
                // Trim trailing backslash from namespace (PHP syntax: use App\{Foo, Bar})
                let namespace = cap.get(3).unwrap().as_str().trim_end_matches('\\');
                let items = cap.get(4).unwrap().as_str();

                // Check not in string
                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                // Parse the items
                let item_list: Vec<&str> = items
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                if item_list.len() <= 1 {
                    continue;
                }

                // Generate individual use statements
                let mut statements = Vec::new();
                for item in item_list {
                    // Handle aliases: Class as Alias
                    let full_name = if item.contains(" as ") {
                        let parts: Vec<&str> = item.splitn(2, " as ").collect();
                        format!("{}\\{} as {}", namespace, parts[0].trim(), parts[1].trim())
                    } else {
                        format!("{}\\{}", namespace, item)
                    };
                    statements.push(format!("{}{}{};", indent, use_prefix, full_name));
                }

                let replacement = statements.join(line_ending);

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    replacement,
                    "Split grouped import into separate statements".to_string(),
                    "single_import_per_statement",
                ));
            }
        }

        // Also match comma-separated imports: use Foo, Bar, Baz;
        let comma_re = Regex::new(r"(?m)^([ \t]*)(use\s+(?:function\s+|const\s+)?)([^;{]+,\s*[^;{]+);").unwrap();

        for cap in comma_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let use_prefix = cap.get(2).unwrap().as_str();
            let names = cap.get(3).unwrap().as_str();

            // Skip if already processed as grouped import
            if names.contains('{') || names.contains('}') {
                continue;
            }

            // Check not in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            let name_list: Vec<&str> = names
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if name_list.len() <= 1 {
                continue;
            }

            let statements: Vec<String> = name_list
                .iter()
                .map(|name| format!("{}{}{};", indent, use_prefix, name))
                .collect();

            let replacement = statements.join(line_ending);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Split comma-separated imports into separate statements".to_string(),
                "single_import_per_statement",
            ));
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        SingleImportPerStatementFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_single_import_unchanged() {
        let source = "<?php\n\nuse App\\Model;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_grouped_import() {
        let source = "<?php\n\nuse App\\{Model, Controller, View};\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use App\\Model;"));
        assert!(edits[0].replacement.contains("use App\\Controller;"));
        assert!(edits[0].replacement.contains("use App\\View;"));
    }

    #[test]
    fn test_grouped_with_alias() {
        let source = "<?php\n\nuse App\\{Model, Controller as Ctrl};\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use App\\Model;"));
        assert!(edits[0].replacement.contains("use App\\Controller as Ctrl;"));
    }

    #[test]
    fn test_grouped_function_import() {
        let source = "<?php\n\nuse function App\\{foo, bar};\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use function App\\foo;"));
        assert!(edits[0].replacement.contains("use function App\\bar;"));
    }

    #[test]
    fn test_grouped_const_import() {
        let source = "<?php\n\nuse const App\\{FOO, BAR};\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use const App\\FOO;"));
        assert!(edits[0].replacement.contains("use const App\\BAR;"));
    }

    #[test]
    fn test_comma_separated() {
        let source = "<?php\n\nuse App\\Model, App\\View;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use App\\Model;"));
        assert!(edits[0].replacement.contains("use App\\View;"));
    }

    #[test]
    fn test_preserves_indent() {
        let source = "<?php\n\n    use App\\{Model, View};\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        for line in edits[0].replacement.lines() {
            assert!(line.starts_with("    use "));
        }
    }

    #[test]
    fn test_group_to_single_imports_false() {
        // When group_to_single_imports is false (PSR-12), grouped imports should NOT be split
        let source = "<?php\n\nuse App\\{Model, Controller, View};\n";
        let mut options = std::collections::HashMap::new();
        options.insert("group_to_single_imports".to_string(), ConfigValue::Bool(false));

        let edits = SingleImportPerStatementFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            options,
            ..Default::default()
        });

        // No edits - grouped imports should remain grouped
        assert!(edits.is_empty(), "Expected no edits when group_to_single_imports is false");
    }

    #[test]
    fn test_comma_separated_always_split() {
        // Comma-separated imports (use A, B;) should ALWAYS be split, regardless of option
        let source = "<?php\n\nuse App\\Model, App\\View;\n";
        let mut options = std::collections::HashMap::new();
        options.insert("group_to_single_imports".to_string(), ConfigValue::Bool(false));

        let edits = SingleImportPerStatementFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            options,
            ..Default::default()
        });

        // Comma-separated imports should still be split
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use App\\Model;"));
        assert!(edits[0].replacement.contains("use App\\View;"));
    }
}
