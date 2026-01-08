//! Ensure full opening PHP tag

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures PHP files use full opening tag `<?php` instead of short tag `<?`
pub struct FullOpeningTagFixer;

impl Fixer for FullOpeningTagFixer {
    fn name(&self) -> &'static str {
        "full_opening_tag"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "full_opening_tag"
    }

    fn description(&self) -> &'static str {
        "Use full PHP opening tag <?php instead of short tag <?"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find short PHP tags that aren't <?php or <?=
        let bytes = source.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' && i + 1 < bytes.len() && bytes[i + 1] == b'?' {
                // Found <?
                let after_tag_start = i + 2;

                // Check what follows <?
                if after_tag_start < bytes.len() {
                    let next_char = bytes[after_tag_start];

                    // Skip <?php (already full tag)
                    if source[after_tag_start..].starts_with("php") {
                        i = after_tag_start + 3;
                        continue;
                    }

                    // Skip <?= (short echo tag, valid)
                    if next_char == b'=' {
                        i = after_tag_start + 1;
                        continue;
                    }

                    // Skip <?xml (not PHP)
                    if source[after_tag_start..].starts_with("xml") {
                        i = after_tag_start + 3;
                        continue;
                    }

                    // This is a short tag <? that should be <?php
                    // Determine if we need a space after
                    let needs_space = next_char != b' '
                        && next_char != b'\t'
                        && next_char != b'\n'
                        && next_char != b'\r';

                    let replacement = if needs_space { "<?php " } else { "<?php" };

                    edits.push(edit_with_rule(
                        i,
                        after_tag_start,
                        replacement.to_string(),
                        "Convert short PHP tag to full opening tag".to_string(),
                        "full_opening_tag",
                    ));
                }
            }
            i += 1;
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        FullOpeningTagFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_full_tag_unchanged() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_short_echo_tag_unchanged() {
        let edits = check("<?= $var ?>");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_xml_tag_unchanged() {
        let edits = check("<?xml version=\"1.0\"?>\n<?php echo 'hi'; ?>");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_short_tag_with_space() {
        let source = "<? echo 'hi'; ?>";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "<?php");
    }

    #[test]
    fn test_short_tag_with_newline() {
        let source = "<?\necho 'hi';\n?>";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "<?php");
    }

    #[test]
    fn test_short_tag_immediate_code() {
        let source = "<?echo 'hi';";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "<?php ");
    }

    #[test]
    fn test_multiple_short_tags() {
        let source = "<? $a = 1; ?>\n<? $b = 2; ?>";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }
}
