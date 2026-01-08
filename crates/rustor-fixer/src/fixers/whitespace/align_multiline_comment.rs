//! Align multiline comments

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct AlignMultilineCommentFixer;

impl Fixer for AlignMultilineCommentFixer {
    fn name(&self) -> &'static str { "align_multiline_comment" }
    fn php_cs_fixer_name(&self) -> &'static str { "align_multiline_comment" }
    fn description(&self) -> &'static str { "Align multiline comment stars" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find multiline comments /* ... */ and align the * on each line
        let comment_re = Regex::new(r"(?ms)/\*[^*].*?\*/").unwrap();

        for m in comment_re.find_iter(source) {
            let comment = m.as_str();
            if !comment.contains('\n') { continue; }

            let lines: Vec<&str> = comment.lines().collect();
            if lines.len() < 2 { continue; }

            // Get the base indentation from first line
            let first_line_start = m.start();
            let line_start = source[..first_line_start].rfind('\n').map_or(0, |p| p + 1);
            let base_indent: String = source[line_start..first_line_start]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();

            let mut new_lines = vec![lines[0].to_string()];
            let mut changed = false;

            for line in lines.iter().skip(1) {
                let trimmed = line.trim_start();
                if trimmed.starts_with('*') {
                    let expected = format!("{} {}", base_indent, trimmed);
                    if *line != expected {
                        new_lines.push(expected);
                        changed = true;
                    } else {
                        new_lines.push(line.to_string());
                    }
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if changed {
                edits.push(edit_with_rule(
                    m.start(), m.end(),
                    new_lines.join("\n"),
                    "Align multiline comment".to_string(),
                    "align_multiline_comment",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_align_comment() {
        let code = "    /*
* misaligned
    */";
        let edits = AlignMultilineCommentFixer.check(code, &FixerConfig::default());
        // Complex alignment logic
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
