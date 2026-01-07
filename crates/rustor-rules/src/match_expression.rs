//! Rule: Convert simple switch statements to match expressions (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! switch ($status) {
//!     case 'active': $label = 'Active'; break;
//!     case 'pending': $label = 'Pending'; break;
//!     default: $label = 'Unknown';
//! }
//!
//! // After
//! $label = match($status) {
//!     'active' => 'Active',
//!     'pending' => 'Pending',
//!     default => 'Unknown',
//! };
//! ```
//!
//! Requirements for conversion:
//! - Each case must assign to the same variable
//! - Each case must have exactly one assignment followed by break
//! - No fall-through between cases
//! - Must have at least 2 cases (including default)

use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::ast::Sequence;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for switch statements convertible to match
pub fn check_match_expression<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = MatchExpressionVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct MatchExpressionVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for MatchExpressionVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        if let Statement::Switch(switch) = stmt {
            self.check_switch(switch);
        }
        true // Continue traversal
    }
}

/// Information about a case that can be converted
struct CaseInfo {
    /// The case condition(s) - None for default
    conditions: Vec<String>,
    /// The target variable being assigned to
    target_var: String,
    /// The value being assigned
    value: String,
    /// Whether this is the default case
    is_default: bool,
}

impl<'s> MatchExpressionVisitor<'s> {
    fn check_switch(&mut self, switch: &Switch<'_>) {
        // Extract the condition expression
        let condition_span = switch.expression.span();
        let condition = &self.source[condition_span.start.offset as usize..condition_span.end.offset as usize];

        // Get cases from the switch body
        let switch_cases = match &switch.body {
            SwitchBody::BraceDelimited(body) => &body.cases,
            SwitchBody::ColonDelimited(body) => &body.cases,
        };

        // Analyze all cases
        let cases = match self.analyze_cases(switch_cases) {
            Some(c) => c,
            None => return,
        };

        // Need at least 2 cases for a meaningful match
        if cases.len() < 2 {
            return;
        }

        // All cases must assign to the same variable
        let target_var = match self.find_common_target(&cases) {
            Some(v) => v,
            None => return,
        };

        // Build the match expression
        let mut arms = Vec::new();
        for case in &cases {
            if case.is_default {
                arms.push(format!("    default => {}", case.value));
            } else {
                let conditions = case.conditions.join(", ");
                arms.push(format!("    {} => {}", conditions, case.value));
            }
        }

        let match_expr = format!(
            "{} = match({}) {{\n{},\n}}",
            target_var,
            condition,
            arms.join(",\n")
        );

        let span = switch.span();
        self.edits.push(Edit::new(
            span,
            match_expr,
            "Convert switch to match expression (PHP 8.0+)",
        ));
    }

    /// Analyze switch cases to see if they can be converted to match
    fn analyze_cases(&self, cases: &Sequence<'_, SwitchCase<'_>>) -> Option<Vec<CaseInfo>> {
        let mut result = Vec::new();
        let cases_vec: Vec<_> = cases.iter().collect();
        let mut i = 0;

        while i < cases_vec.len() {
            let _case = cases_vec[i];

            // Check for fall-through (multiple conditions for same body)
            let mut conditions = Vec::new();
            let mut body_case_idx = i;

            // Collect consecutive cases that fall through (empty body)
            while body_case_idx < cases_vec.len() {
                let current = cases_vec[body_case_idx];

                // Add this case's condition
                if let SwitchCase::Expression(case_stmt) = current {
                    let cond_span = case_stmt.expression.span();
                    let cond = self.source[cond_span.start.offset as usize..cond_span.end.offset as usize].to_string();
                    conditions.push(cond);

                    // Check if this case has an empty body (fall-through)
                    if self.is_empty_case_body(&case_stmt.statements) {
                        body_case_idx += 1;
                        continue;
                    }
                } else if let SwitchCase::Default(_) = current {
                    // Default case - should be last
                    if !conditions.is_empty() {
                        // Fall-through to default is not supported
                        return None;
                    }
                }
                break;
            }

            // Now analyze the actual case with body
            let body_case = cases_vec[body_case_idx];

            match body_case {
                SwitchCase::Expression(case_stmt) => {
                    // If we haven't collected this case's condition yet
                    if conditions.is_empty() || body_case_idx > i {
                        let cond_span = case_stmt.expression.span();
                        let cond = self.source[cond_span.start.offset as usize..cond_span.end.offset as usize].to_string();
                        if !conditions.contains(&cond) {
                            conditions.push(cond);
                        }
                    }

                    let (var, value) = self.extract_assignment_and_break(&case_stmt.statements)?;
                    result.push(CaseInfo {
                        conditions,
                        target_var: var,
                        value,
                        is_default: false,
                    });
                }
                SwitchCase::Default(default_stmt) => {
                    let (var, value) = self.extract_assignment_from_default(&default_stmt.statements)?;
                    result.push(CaseInfo {
                        conditions: vec![],
                        target_var: var,
                        value,
                        is_default: true,
                    });
                }
            }

            i = body_case_idx + 1;
        }

        Some(result)
    }

    /// Check if a case body is empty (for fall-through detection)
    fn is_empty_case_body(&self, statements: &Sequence<'_, Statement<'_>>) -> bool {
        statements.is_empty()
    }

    /// Extract assignment variable and value from case body, ensuring it ends with break
    fn extract_assignment_and_break(&self, statements: &Sequence<'_, Statement<'_>>) -> Option<(String, String)> {
        // Should have exactly 2 statements: assignment and break
        // Or 1 statement if it's a block containing assignment + break
        let stmts: Vec<_> = statements.iter().collect();

        if stmts.len() == 2 {
            // First should be expression statement with assignment
            let (var, value) = self.extract_assignment(stmts[0])?;

            // Second should be break
            if !matches!(stmts[1], Statement::Break(_)) {
                return None;
            }

            return Some((var, value));
        }

        if stmts.len() == 1 {
            // Could be just a break (fall-through case handled elsewhere)
            // Or a block statement
            if let Statement::Block(block) = stmts[0] {
                return self.extract_assignment_and_break(&block.statements);
            }
        }

        None
    }

    /// Extract assignment from default case (break is optional)
    fn extract_assignment_from_default(&self, statements: &Sequence<'_, Statement<'_>>) -> Option<(String, String)> {
        if statements.is_empty() {
            return None;
        }

        let stmts: Vec<_> = statements.iter().collect();

        // First statement should be assignment
        let (var, value) = self.extract_assignment(stmts[0])?;

        // Optional break
        if stmts.len() > 1 {
            if stmts.len() == 2 && matches!(stmts[1], Statement::Break(_)) {
                return Some((var, value));
            }
            // More than expected statements
            return None;
        }

        Some((var, value))
    }

    /// Extract variable and value from an assignment statement
    fn extract_assignment(&self, stmt: &Statement<'_>) -> Option<(String, String)> {
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = expr_stmt.expression {
                // Must be simple assignment (=), not compound (+=, etc.)
                if !matches!(&assign.operator, AssignmentOperator::Assign(_)) {
                    return None;
                }

                // LHS must be a simple variable
                let var_span = assign.lhs.span();
                let var = self.source[var_span.start.offset as usize..var_span.end.offset as usize].to_string();

                // Get the RHS value
                let value_span = assign.rhs.span();
                let value = self.source[value_span.start.offset as usize..value_span.end.offset as usize].to_string();

                return Some((var, value));
            }
        }
        None
    }

    /// Find the common target variable across all cases
    fn find_common_target(&self, cases: &[CaseInfo]) -> Option<String> {
        if cases.is_empty() {
            return None;
        }

        let first_target = &cases[0].target_var;

        // All cases must assign to the same variable
        for case in cases.iter().skip(1) {
            if &case.target_var != first_target {
                return None;
            }
        }

        Some(first_target.clone())
    }
}

pub struct MatchExpressionRule;

impl Rule for MatchExpressionRule {
    fn name(&self) -> &'static str {
        "match_expression"
    }

    fn description(&self) -> &'static str {
        "Convert simple switch to match expression"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_match_expression(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_match_expression(program, source)
    }

    #[test]
    fn test_match_rule_exists() {
        let rule = MatchExpressionRule;
        assert_eq!(rule.name(), "match_expression");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php80));
    }

    #[test]
    fn test_simple_switch_converted() {
        let source = r#"<?php
switch ($status) {
    case 'active': $label = 'Active'; break;
    case 'pending': $label = 'Pending'; break;
    default: $label = 'Unknown';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("match($status)"));
        assert!(edits[0].replacement.contains("'active' => 'Active'"));
        assert!(edits[0].replacement.contains("'pending' => 'Pending'"));
        assert!(edits[0].replacement.contains("default => 'Unknown'"));
    }

    #[test]
    fn test_switch_with_variables() {
        let source = r#"<?php
switch ($type) {
    case 1: $result = $a; break;
    case 2: $result = $b; break;
    default: $result = $c;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$result = match($type)"));
    }

    #[test]
    fn test_skip_switch_with_side_effects() {
        // Switch with function calls shouldn't be converted
        let source = r#"<?php
switch ($status) {
    case 'active': echo 'Active'; break;
    case 'pending': echo 'Pending'; break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_switch_with_multiple_statements() {
        // Switch cases with multiple statements shouldn't be converted
        let source = r#"<?php
switch ($status) {
    case 'active':
        $label = 'Active';
        $count++;
        break;
    default: $label = 'Unknown';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_target_variables() {
        // Different target variables should not be converted
        let source = r#"<?php
switch ($status) {
    case 'active': $label = 'Active'; break;
    case 'pending': $other = 'Pending'; break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_switch_without_default() {
        // Switch without default can still be converted
        let source = r#"<?php
switch ($status) {
    case 'active': $label = 'Active'; break;
    case 'pending': $label = 'Pending'; break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_single_case() {
        // Single case is not meaningful for match
        let source = r#"<?php
switch ($status) {
    case 'active': $label = 'Active'; break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_compound_assignment() {
        // Compound assignment (+=, .=, etc.) shouldn't be converted
        let source = r#"<?php
switch ($status) {
    case 'active': $count += 1; break;
    case 'pending': $count += 2; break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_switch_with_expressions() {
        let source = r#"<?php
switch ($value) {
    case 1 + 1: $x = 'two'; break;
    case 2 + 2: $x = 'four'; break;
    default: $x = 'other';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
