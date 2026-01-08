//! Space after semicolon in for loops

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SpaceAfterSemicolonFixer;

impl Fixer for SpaceAfterSemicolonFixer {
    fn name(&self) -> &'static str { "space_after_semicolon" }
    fn php_cs_fixer_name(&self) -> &'static str { "space_after_semicolon" }
    fn description(&self) -> &'static str { "Space after semicolon in for loops" }
    fn priority(&self) -> i32 { 25 } // Run early

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find for loops: for(init;cond;incr) or for (init;cond;incr)
        let for_re = Regex::new(r"(?i)\bfor\s*\(").unwrap();

        for mat in for_re.find_iter(source) {
            let start = mat.end(); // Position after opening (

            // Find the closing ) by counting parentheses
            let rest = &source[start..];
            let mut depth = 1;
            let mut end = start;

            for (i, c) in rest.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if end > start {
                let for_content = &source[start..end];

                // Fix semicolons without space after: ; followed by non-whitespace
                // But only in for loops where we have exactly 2 semicolons
                let semicolon_count = for_content.matches(';').count();
                if semicolon_count == 2 {
                    // Find and fix each semicolon
                    let chars: Vec<char> = for_content.chars().collect();
                    let len = chars.len();
                    let mut char_offset = 0;

                    for (idx, &c) in chars.iter().enumerate() {
                        if c == ';' {
                            let byte_pos = start + char_offset;

                            // Check if space after is needed
                            let next = if idx + 1 < len { Some(chars[idx + 1]) } else { None };
                            let needs_space_after = next.map(|nc| !nc.is_whitespace()).unwrap_or(false);

                            if needs_space_after {
                                edits.push(edit_with_rule(
                                    byte_pos,
                                    byte_pos + 1,
                                    "; ".to_string(),
                                    "Add space after semicolon in for loop".to_string(),
                                    "space_after_semicolon",
                                ));
                            }
                        }
                        char_offset += c.len_utf8();
                    }
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        SpaceAfterSemicolonFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_missing_space() {
        let edits = check("<?php\nfor ($i=0;$i<10;$i++) {}");
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_has_space() {
        let edits = check("<?php\nfor ($i = 0; $i < 10; $i++) {}");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_partial_spacing() {
        let edits = check("<?php\nfor ($i = 0; $i < 10;$i++) {}");
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_nested_parentheses() {
        let edits = check("<?php\nfor ($i = fn();$i < max();$i++) {}");
        assert_eq!(edits.len(), 2);
    }
}
