//! Check for files not ending with a newline (Level 0)
//!
//! Detects PHP files that don't end with a trailing newline.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for files not ending with a newline
pub struct WhitespaceFileEndCheck;

impl Check for WhitespaceFileEndCheck {
    fn id(&self) -> &'static str {
        "whitespace.fileEnd"
    }

    fn description(&self) -> &'static str {
        "Detects files not ending with a newline"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, _program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut issues = Vec::new();

        if !ctx.source.is_empty() && !ctx.source.ends_with('\n') {
            let line = ctx.source.lines().count();
            issues.push(
                Issue::error(
                    "whitespace.fileEnd",
                    "File ends without a trailing newline.".to_string(),
                    ctx.file_path.to_path_buf(),
                    line,
                    1,
                )
                .with_identifier("whitespace.fileEnd"),
            );
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_file_end_check_level() {
        let check = WhitespaceFileEndCheck;
        assert_eq!(check.level(), 0);
    }
}
