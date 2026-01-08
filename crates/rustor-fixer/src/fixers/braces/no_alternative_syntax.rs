//! No alternative syntax fixer
//!
//! Replaces alternative syntax (if/endif, foreach/endforeach, etc.) with braces.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Replaces alternative control structure syntax with braces
pub struct NoAlternativeSyntaxFixer;

impl Fixer for NoAlternativeSyntaxFixer {
    fn name(&self) -> &'static str {
        "no_alternative_syntax"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_alternative_syntax"
    }

    fn description(&self) -> &'static str {
        "Replace alternative syntax (endif, endforeach, etc.) with braces"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // This is a complex transformation that requires careful handling.
        // For now, we'll flag the simple cases.

        // Match `if (...):` pattern
        let if_colon_re = Regex::new(r"\bif\s*\([^)]+\)\s*:").unwrap();
        for m in if_colon_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            // Just flag for now - full transformation is complex
            let colon_pos = m.as_str().rfind(':').unwrap();
            let abs_pos = m.start() + colon_pos;
            edits.push(edit_with_rule(
                abs_pos,
                abs_pos + 1,
                " {".to_string(),
                "Use braces instead of alternative syntax".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `endif;`
        let endif_re = Regex::new(r"\bendif\s*;").unwrap();
        for m in endif_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "}".to_string(),
                "Use braces instead of endif".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `foreach (...):` pattern
        let foreach_colon_re = Regex::new(r"\bforeach\s*\([^)]+\)\s*:").unwrap();
        for m in foreach_colon_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            let colon_pos = m.as_str().rfind(':').unwrap();
            let abs_pos = m.start() + colon_pos;
            edits.push(edit_with_rule(
                abs_pos,
                abs_pos + 1,
                " {".to_string(),
                "Use braces instead of alternative syntax".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `endforeach;`
        let endforeach_re = Regex::new(r"\bendforeach\s*;").unwrap();
        for m in endforeach_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "}".to_string(),
                "Use braces instead of endforeach".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `while (...):` pattern
        let while_colon_re = Regex::new(r"\bwhile\s*\([^)]+\)\s*:").unwrap();
        for m in while_colon_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            let colon_pos = m.as_str().rfind(':').unwrap();
            let abs_pos = m.start() + colon_pos;
            edits.push(edit_with_rule(
                abs_pos,
                abs_pos + 1,
                " {".to_string(),
                "Use braces instead of alternative syntax".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `endwhile;`
        let endwhile_re = Regex::new(r"\bendwhile\s*;").unwrap();
        for m in endwhile_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "}".to_string(),
                "Use braces instead of endwhile".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `for (...):` pattern
        let for_colon_re = Regex::new(r"\bfor\s*\([^)]+\)\s*:").unwrap();
        for m in for_colon_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            let colon_pos = m.as_str().rfind(':').unwrap();
            let abs_pos = m.start() + colon_pos;
            edits.push(edit_with_rule(
                abs_pos,
                abs_pos + 1,
                " {".to_string(),
                "Use braces instead of alternative syntax".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `endfor;`
        let endfor_re = Regex::new(r"\bendfor\s*;").unwrap();
        for m in endfor_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "}".to_string(),
                "Use braces instead of endfor".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `switch (...):` pattern
        let switch_colon_re = Regex::new(r"\bswitch\s*\([^)]+\)\s*:").unwrap();
        for m in switch_colon_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            let colon_pos = m.as_str().rfind(':').unwrap();
            let abs_pos = m.start() + colon_pos;
            edits.push(edit_with_rule(
                abs_pos,
                abs_pos + 1,
                " {".to_string(),
                "Use braces instead of alternative syntax".to_string(),
                "no_alternative_syntax",
            ));
        }

        // Match `endswitch;`
        let endswitch_re = Regex::new(r"\bendswitch\s*;").unwrap();
        for m in endswitch_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "}".to_string(),
                "Use braces instead of endswitch".to_string(),
                "no_alternative_syntax",
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

    fn check(source: &str) -> Vec<Edit> {
        NoAlternativeSyntaxFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nif ($a) {\n    echo 1;\n}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_if_endif() {
        let source = "<?php\nif ($a):\n    echo 1;\nendif;";
        let edits = check(source);
        assert_eq!(edits.len(), 2); // colon -> { and endif -> }
    }

    #[test]
    fn test_foreach_endforeach() {
        let source = "<?php\nforeach ($arr as $item):\n    echo $item;\nendforeach;";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_while_endwhile() {
        let source = "<?php\nwhile ($a):\n    echo 1;\nendwhile;";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_for_endfor() {
        let source = "<?php\nfor ($i = 0; $i < 10; $i++):\n    echo $i;\nendfor;";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_switch_endswitch() {
        let source = "<?php\nswitch ($a):\n    case 1:\n        break;\nendswitch;";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'if ($x): endif;';";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
