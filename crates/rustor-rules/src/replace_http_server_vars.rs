//! Rule: Replace old $HTTP_*_VARS variables with superglobals
//!
//! Since PHP 5.3, the old long array names are deprecated.
//!
//! Transformations:
//! - `$HTTP_SERVER_VARS` → `$_SERVER`
//! - `$HTTP_GET_VARS` → `$_GET`
//! - `$HTTP_POST_VARS` → `$_POST`
//! - `$HTTP_POST_FILES` → `$_FILES`
//! - `$HTTP_SESSION_VARS` → `$_SESSION`
//! - `$HTTP_ENV_VARS` → `$_ENV`
//! - `$HTTP_COOKIE_VARS` → `$_COOKIE`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for old HTTP_*_VARS variables
pub fn check_replace_http_server_vars<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ReplaceHttpServerVarsVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ReplaceHttpServerVarsVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

/// Old variable names to new superglobal names
const VARIABLE_RENAME_MAP: &[(&str, &str)] = &[
    ("HTTP_SERVER_VARS", "_SERVER"),
    ("HTTP_GET_VARS", "_GET"),
    ("HTTP_POST_VARS", "_POST"),
    ("HTTP_POST_FILES", "_FILES"),
    ("HTTP_SESSION_VARS", "_SESSION"),
    ("HTTP_ENV_VARS", "_ENV"),
    ("HTTP_COOKIE_VARS", "_COOKIE"),
];

impl<'a, 's> Visitor<'a> for ReplaceHttpServerVarsVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Variable(var) = expr {
            if let Some(edit) = try_replace_http_var(var, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to replace old HTTP_*_VARS variable
fn try_replace_http_var(var: &Variable<'_>, source: &str) -> Option<Edit> {
    // Get the variable name
    let var_span = var.span();
    let var_text = &source[var_span.start.offset as usize..var_span.end.offset as usize];

    // Remove the $ prefix to get the name
    let var_name = var_text.strip_prefix('$')?;

    // Check if it matches any old HTTP variable name
    for (old_name, new_name) in VARIABLE_RENAME_MAP {
        if var_name.eq_ignore_ascii_case(old_name) {
            return Some(Edit::new(
                var_span,
                format!("${}", new_name),
                "Replace old HTTP variable with superglobal",
            ));
        }
    }

    None
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct ReplaceHttpServerVarsRule;

impl Rule for ReplaceHttpServerVarsRule {
    fn name(&self) -> &'static str {
        "replace_http_server_vars"
    }

    fn description(&self) -> &'static str {
        "Replace old $HTTP_*_VARS variables with superglobals"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_replace_http_server_vars(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php54)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_replace_http_server_vars(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== HTTP_SERVER_VARS ====================

    #[test]
    fn test_http_server_vars() {
        let source = "<?php $x = $HTTP_SERVER_VARS;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x = $_SERVER;");
    }

    #[test]
    fn test_http_server_vars_access() {
        let source = "<?php $ip = $HTTP_SERVER_VARS['REMOTE_ADDR'];";
        assert_eq!(transform(source), "<?php $ip = $_SERVER['REMOTE_ADDR'];");
    }

    // ==================== HTTP_GET_VARS ====================

    #[test]
    fn test_http_get_vars() {
        let source = "<?php $id = $HTTP_GET_VARS['id'];";
        assert_eq!(transform(source), "<?php $id = $_GET['id'];");
    }

    // ==================== HTTP_POST_VARS ====================

    #[test]
    fn test_http_post_vars() {
        let source = "<?php $name = $HTTP_POST_VARS['name'];";
        assert_eq!(transform(source), "<?php $name = $_POST['name'];");
    }

    // ==================== HTTP_POST_FILES ====================

    #[test]
    fn test_http_post_files() {
        let source = "<?php $file = $HTTP_POST_FILES['upload'];";
        assert_eq!(transform(source), "<?php $file = $_FILES['upload'];");
    }

    // ==================== HTTP_SESSION_VARS ====================

    #[test]
    fn test_http_session_vars() {
        let source = "<?php $user = $HTTP_SESSION_VARS['user'];";
        assert_eq!(transform(source), "<?php $user = $_SESSION['user'];");
    }

    // ==================== HTTP_ENV_VARS ====================

    #[test]
    fn test_http_env_vars() {
        let source = "<?php $path = $HTTP_ENV_VARS['PATH'];";
        assert_eq!(transform(source), "<?php $path = $_ENV['PATH'];");
    }

    // ==================== HTTP_COOKIE_VARS ====================

    #[test]
    fn test_http_cookie_vars() {
        let source = "<?php $token = $HTTP_COOKIE_VARS['token'];";
        assert_eq!(transform(source), "<?php $token = $_COOKIE['token'];");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$get = $HTTP_GET_VARS;
$post = $HTTP_POST_VARS;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_new_superglobal() {
        let source = "<?php $x = $_SERVER;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_regular_variable() {
        let source = "<?php $http_server = 'localhost';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
