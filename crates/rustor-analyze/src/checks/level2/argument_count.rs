//! Check for wrong argument counts in function/method calls
//!
//! This is a placeholder for future argument count checking.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for function/method calls with wrong number of arguments
///
/// This check is intentionally conservative - it only reports errors when
/// we can be certain about the expected argument count.
pub struct ArgumentCountCheck;

impl Check for ArgumentCountCheck {
    fn id(&self) -> &'static str {
        "argument.count"
    }

    fn description(&self) -> &'static str {
        "Detects function/method calls with wrong argument count"
    }

    fn level(&self) -> u8 {
        2
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // This check requires a full symbol table with function signatures to be useful.
        // Without knowing the parameters of functions and methods (including default values
        // and variadic parameters), we can't accurately check argument counts.
        //
        // Future implementation will use the symbol table to provide
        // accurate argument count errors.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_count_check_id() {
        let check = ArgumentCountCheck;
        assert_eq!(check.id(), "argument.count");
        assert_eq!(check.level(), 2);
    }
}
