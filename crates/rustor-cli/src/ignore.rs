//! Inline ignore directive support for rustor
//!
//! Supports the following comment directives:
//! - `// rustor-ignore` - Ignore all rules for the next line
//! - `// rustor-ignore: rule1, rule2` - Ignore specific rules for the next line
//! - `// rustor-ignore-line` - Ignore all rules for the current line (inline comment)
//! - `// rustor-ignore-line: rule1, rule2` - Ignore specific rules for the current line
//! - `// rustor-ignore-file` - Ignore all rules for the entire file
//! - `// rustor-ignore-file: rule1, rule2` - Ignore specific rules for the entire file
//!
//! Also supports block comments: `/* rustor-ignore */`

use std::collections::{HashMap, HashSet};

/// Parsed ignore directives from source code
#[derive(Debug, Default)]
pub struct IgnoreDirectives {
    /// Rules ignored for the entire file (empty set means ignore all)
    file_ignores: Option<HashSet<String>>,
    /// Rules ignored for specific lines (line number -> rules, empty set means ignore all)
    line_ignores: HashMap<usize, HashSet<String>>,
}

impl IgnoreDirectives {
    /// Parse ignore directives from PHP source code
    pub fn parse(source: &str) -> Self {
        let mut directives = IgnoreDirectives::default();
        let lines: Vec<&str> = source.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1; // 1-based line numbers
            let trimmed = line.trim();

            // Check for file-level ignores first
            if let Some(rules) = parse_ignore_file(trimmed) {
                if rules.is_empty() {
                    // Ignore all rules for entire file
                    directives.file_ignores = Some(HashSet::new());
                } else {
                    // Ignore specific rules for entire file
                    let existing = directives.file_ignores.get_or_insert_with(HashSet::new);
                    for rule in rules {
                        existing.insert(rule);
                    }
                }
                continue;
            }

            // Check for inline ignore (rustor-ignore-line)
            if let Some(rules) = parse_ignore_line(trimmed) {
                let entry = directives.line_ignores.entry(line_num).or_default();
                if rules.is_empty() {
                    // Empty means ignore all - keep it empty
                    entry.clear();
                } else {
                    for rule in rules {
                        entry.insert(rule);
                    }
                }
                continue;
            }

            // Check for next-line ignore (rustor-ignore)
            if let Some(rules) = parse_ignore_next(trimmed) {
                let next_line = line_num + 1;
                let entry = directives.line_ignores.entry(next_line).or_default();
                if rules.is_empty() {
                    entry.clear();
                } else {
                    for rule in rules {
                        entry.insert(rule);
                    }
                }
            }
        }

        directives
    }

    /// Check if a specific rule should be ignored at a given line
    pub fn should_ignore(&self, line: usize, rule: &str) -> bool {
        // Check file-level ignores
        if let Some(ref file_rules) = self.file_ignores {
            if file_rules.is_empty() || file_rules.contains(rule) {
                return true;
            }
        }

        // Check line-level ignores
        if let Some(line_rules) = self.line_ignores.get(&line) {
            if line_rules.is_empty() || line_rules.contains(rule) {
                return true;
            }
        }

        false
    }

    /// Check if there are any ignore directives
    pub fn has_any(&self) -> bool {
        self.file_ignores.is_some() || !self.line_ignores.is_empty()
    }
}

/// Parse `// rustor-ignore-file` or `// rustor-ignore-file: rule1, rule2`
fn parse_ignore_file(line: &str) -> Option<Vec<String>> {
    // Match both // and /* */ style comments
    let patterns = [
        "// rustor-ignore-file",
        "/* rustor-ignore-file",
        "# rustor-ignore-file",  // For PHP's # comment style
    ];

    for pattern in patterns {
        if let Some(rest) = line.strip_prefix(pattern) {
            return Some(parse_rule_list(rest));
        }
    }

    None
}

/// Parse `// rustor-ignore-line` or `// rustor-ignore-line: rule1, rule2` (inline)
fn parse_ignore_line(line: &str) -> Option<Vec<String>> {
    // Look for inline comment containing rustor-ignore-line
    let patterns = [
        "// rustor-ignore-line",
        "/* rustor-ignore-line",
        "# rustor-ignore-line",
    ];

    for pattern in patterns {
        if let Some(pos) = line.find(pattern) {
            let rest = &line[pos + pattern.len()..];
            return Some(parse_rule_list(rest));
        }
    }

    None
}

/// Parse `// rustor-ignore` or `// rustor-ignore: rule1, rule2` (for next line)
fn parse_ignore_next(line: &str) -> Option<Vec<String>> {
    // Must be the only content on the line (not inline)
    let patterns = [
        "// rustor-ignore",
        "/* rustor-ignore",
        "# rustor-ignore",
    ];

    for pattern in patterns {
        if let Some(rest) = line.strip_prefix(pattern) {
            // Make sure it's not "rustor-ignore-file" or "rustor-ignore-line"
            if rest.starts_with("-file") || rest.starts_with("-line") {
                continue;
            }
            return Some(parse_rule_list(rest));
        }
    }

    None
}

/// Parse rule list from `: rule1, rule2` or ` */`
fn parse_rule_list(rest: &str) -> Vec<String> {
    let rest = rest.trim();

    // Handle end of block comment
    let rest = rest.trim_end_matches("*/").trim();

    // Check for rule list after colon
    if let Some(rules_str) = rest.strip_prefix(':') {
        let rules: Vec<String> = rules_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return rules;
    }

    // No colon means ignore all rules
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_file_all() {
        let source = "<?php\n// rustor-ignore-file\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
        assert!(directives.should_ignore(3, "array_push"));
    }

    #[test]
    fn test_ignore_file_specific() {
        let source = "<?php\n// rustor-ignore-file: is_null, array_push\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
        assert!(directives.should_ignore(3, "array_push"));
        assert!(!directives.should_ignore(3, "sizeof"));
    }

    #[test]
    fn test_ignore_next_line_all() {
        let source = "<?php\n// rustor-ignore\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
        assert!(!directives.should_ignore(2, "is_null")); // The comment line itself
    }

    #[test]
    fn test_ignore_next_line_specific() {
        let source = "<?php\n// rustor-ignore: is_null\n$x = is_null($y);\narray_push($arr, $v);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
        assert!(!directives.should_ignore(3, "array_push"));
        assert!(!directives.should_ignore(4, "is_null"));
        assert!(!directives.should_ignore(4, "array_push"));
    }

    #[test]
    fn test_ignore_line_inline() {
        let source = "<?php\n$x = is_null($y); // rustor-ignore-line";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(2, "is_null"));
        assert!(directives.should_ignore(2, "array_push"));
    }

    #[test]
    fn test_ignore_line_inline_specific() {
        let source = "<?php\n$x = is_null($y); // rustor-ignore-line: is_null";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(2, "is_null"));
        assert!(!directives.should_ignore(2, "array_push"));
    }

    #[test]
    fn test_block_comment_style() {
        let source = "<?php\n/* rustor-ignore */\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
    }

    #[test]
    fn test_block_comment_with_rules() {
        let source = "<?php\n/* rustor-ignore: is_null */\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
        assert!(!directives.should_ignore(3, "array_push"));
    }

    #[test]
    fn test_hash_comment_style() {
        let source = "<?php\n# rustor-ignore\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, "is_null"));
    }

    #[test]
    fn test_no_directives() {
        let source = "<?php\n$x = is_null($y);";
        let directives = IgnoreDirectives::parse(source);
        assert!(!directives.should_ignore(2, "is_null"));
        assert!(!directives.has_any());
    }

    #[test]
    fn test_multiple_directives() {
        let source = r#"<?php
// rustor-ignore-file: is_null
// rustor-ignore
$x = is_null($y);
array_push($arr, $v);
$z = is_null($w); // rustor-ignore-line: is_null
"#;
        let directives = IgnoreDirectives::parse(source);
        // Line 4: is_null ignored by file, array_push ignored by previous line
        assert!(directives.should_ignore(4, "is_null"));
        // Line 4: should be ignored by "// rustor-ignore" on line 3
        assert!(directives.should_ignore(4, "array_push"));
        // Line 5: only is_null from file ignore
        assert!(directives.should_ignore(5, "is_null"));
        assert!(!directives.should_ignore(5, "array_push"));
        // Line 6: is_null from both file and line ignore
        assert!(directives.should_ignore(6, "is_null"));
    }
}
