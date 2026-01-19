//! Check for undefined variables

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct UndefinedVariableCheck;

impl Check for UndefinedVariableCheck {
    fn id(&self) -> &'static str {
        "undefined.variable"
    }

    fn description(&self) -> &'static str {
        "Detects use of undefined variables"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = VariableAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            scopes: vec![Scope::new()],
            issues: Vec::new(),
            condition_assignments: HashMap::new(),
            current_conditions: Vec::new(),
            inverse_condition_assignments: HashMap::new(),
        };

        // Add superglobals to the global scope
        analyzer.define_superglobals();

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

/// A scope containing defined variables
#[derive(Debug, Clone)]
struct Scope {
    /// Variables that are definitely defined in all code paths
    defined: HashSet<String>,
    /// Variables that are possibly defined (in some but not all code paths)
    possibly_defined: HashSet<String>,
    /// Whether this is a closure scope (closures don't inherit parent scope unless via `use`)
    is_closure: bool,
    /// Variables inherited via closure `use` clause
    inherited: HashSet<String>,
    /// Whether $this is available in this scope (inside a class method)
    has_this: bool,
}

impl Scope {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            possibly_defined: HashSet::new(),
            is_closure: false,
            inherited: HashSet::new(),
            has_this: false,
        }
    }

    fn closure() -> Self {
        Self {
            defined: HashSet::new(),
            possibly_defined: HashSet::new(),
            is_closure: true,
            inherited: HashSet::new(),
            has_this: false,
        }
    }

    fn define(&mut self, name: String) {
        // If we define a variable, it moves from possibly_defined to defined
        self.possibly_defined.remove(&name);
        self.defined.insert(name);
    }

    fn define_possibly(&mut self, name: String) {
        // Only add to possibly_defined if not already definitely defined
        if !self.defined.contains(&name) {
            self.possibly_defined.insert(name);
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name) || self.inherited.contains(name)
    }

    fn is_possibly_defined(&self, name: &str) -> bool {
        self.possibly_defined.contains(name)
    }

    /// Get snapshot of currently defined variables (for branch analysis)
    fn snapshot(&self) -> HashSet<String> {
        self.defined.clone()
    }
}

struct VariableAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    scopes: Vec<Scope>,
    issues: Vec<Issue>,
    /// Condition correlation tracking: maps condition text to variables assigned under that condition
    condition_assignments: HashMap<String, HashSet<String>>,
    /// Stack of current condition texts (for nested if statements)
    current_conditions: Vec<String>,
    /// Tracks variables assigned under specific conditions at statement level (for inverse condition detection)
    /// Maps normalized condition text -> set of variables assigned in if body
    inverse_condition_assignments: HashMap<String, HashSet<String>>,
}

impl<'s> VariableAnalyzer<'s> {
    fn define_superglobals(&mut self) {
        let superglobals = [
            "$_GET", "$_POST", "$_REQUEST", "$_SERVER", "$_SESSION", "$_COOKIE",
            "$_FILES", "$_ENV", "$GLOBALS", "$argc", "$argv",
        ];
        for var in superglobals {
            self.current_scope_mut().define(var.to_string());
        }
    }

    fn current_scope(&self) -> &Scope {
        self.scopes.last().unwrap()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn push_method_scope(&mut self) {
        let mut scope = Scope::new();
        scope.has_this = true;
        scope.define("$this".to_string());
        self.scopes.push(scope);
    }

    fn push_closure_scope(&mut self) {
        let mut scope = Scope::closure();
        // In PHP 5.4+, closures automatically bind $this from enclosing class method
        if self.has_this_in_scope() {
            scope.has_this = true;
            scope.define("$this".to_string());
        }
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Check if $this is available in the current scope chain
    fn has_this_in_scope(&self) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.has_this {
                return true;
            }
        }
        false
    }

    fn is_defined(&self, name: &str) -> bool {
        // For closure scopes, only check the closure scope itself
        if self.current_scope().is_closure {
            return self.current_scope().is_defined(name);
        }

        // Check all scopes from current to global
        for scope in self.scopes.iter().rev() {
            if scope.is_defined(name) {
                return true;
            }
            // Stop at closure boundary unless checking inherited vars
            if scope.is_closure {
                break;
            }
        }
        false
    }

    fn is_possibly_defined(&self, name: &str) -> bool {
        // For closure scopes, only check the closure scope itself
        if self.current_scope().is_closure {
            return self.current_scope().is_possibly_defined(name);
        }

        // Check all scopes from current to global
        for scope in self.scopes.iter().rev() {
            if scope.is_possibly_defined(name) {
                return true;
            }
            // Stop at closure boundary
            if scope.is_closure {
                break;
            }
        }
        false
    }

    fn define(&mut self, name: String) {
        self.current_scope_mut().define(name);
    }

    fn define_possibly(&mut self, name: String) {
        self.current_scope_mut().define_possibly(name);
    }

    /// Get a normalized text representation of a condition expression for correlation tracking
    fn get_condition_text<'a>(&self, expr: &Expression<'a>) -> String {
        let span = expr.span();
        self.source[span.start.offset as usize..span.end.offset as usize].to_string()
    }

    /// Record that a variable was assigned under the current condition(s)
    fn record_conditional_assignment(&mut self, var_name: &str) {
        for cond in &self.current_conditions {
            self.condition_assignments
                .entry(cond.clone())
                .or_default()
                .insert(var_name.to_string());
        }
    }

    /// Check if a variable was assigned under any of the current conditions
    fn is_defined_by_condition_correlation(&self, var_name: &str) -> bool {
        for cond in &self.current_conditions {
            if let Some(vars) = self.condition_assignments.get(cond) {
                if vars.contains(var_name) {
                    return true;
                }
            }
        }
        false
    }

    /// Push a condition onto the condition stack
    fn push_condition<'a>(&mut self, expr: &Expression<'a>) {
        let cond_text = self.get_condition_text(expr);
        self.current_conditions.push(cond_text);
    }

    /// Pop a condition from the condition stack
    fn pop_condition(&mut self) {
        self.current_conditions.pop();
    }

    /// Normalize a condition text for comparison (removes whitespace differences)
    fn normalize_condition(&self, text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Record that variables were assigned under a specific condition (for inverse condition tracking)
    fn record_inverse_condition_assignment(&mut self, condition: &str, vars: &HashSet<String>) {
        let normalized = self.normalize_condition(condition);
        self.inverse_condition_assignments
            .entry(normalized)
            .or_default()
            .extend(vars.iter().cloned());
    }

    /// Extract the base condition from an inverse OR pattern: !$cond || ...
    /// Returns the base condition text if this is an inverse pattern, None otherwise
    fn extract_inverse_base_condition<'a>(&self, condition: &Expression<'a>) -> Option<String> {
        // Pattern: !$cond || ... or (!$cond) || ...
        if let Expression::Binary(binary) = condition {
            let op_span = binary.operator.span();
            let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];

            if op == "||" || op == "or" {
                // Check if LHS is !$something
                if let Some(base_cond) = self.extract_negated_condition(&binary.lhs) {
                    return Some(base_cond);
                }
            }
        }
        None
    }

    /// Extract the base condition from a negated expression: !$cond -> $cond
    fn extract_negated_condition<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::UnaryPrefix(unary) => {
                let op_span = unary.operator.span();
                let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];
                if op == "!" {
                    // Return the inner expression's text
                    let inner_span = unary.operand.span();
                    let inner_text = &self.source[inner_span.start.offset as usize..inner_span.end.offset as usize];
                    return Some(self.normalize_condition(inner_text));
                }
            }
            Expression::Parenthesized(paren) => {
                return self.extract_negated_condition(&paren.expression);
            }
            _ => {}
        }
        None
    }

    /// Check if a variable should be considered defined due to inverse condition pattern
    /// Pattern: if ($cond) { $var = X; } if (!$cond || ...) { $var = Y; } -> $var is defined
    fn check_inverse_condition_defines(&self, var_name: &str) -> bool {
        // Check if the variable was assigned under some condition AND
        // we have a complementary assignment recorded
        for (cond, vars) in &self.inverse_condition_assignments {
            if vars.contains(var_name) {
                // Check if there's a complementary condition that also assigns this var
                // This is tracked separately when we process complementary ifs
                return true;
            }
        }
        false
    }

    /// Get variables newly assigned in a block compared to before state
    fn get_newly_assigned_vars(&self, before: &HashSet<String>, after: &HashSet<String>) -> HashSet<String> {
        after.difference(before).cloned().collect()
    }

    /// Merge branch results: variables defined in all branches become definitely defined,
    /// variables defined in some branches become possibly defined.
    fn merge_branches(&mut self, before_snapshot: &HashSet<String>, branch_snapshots: Vec<HashSet<String>>) {
        if branch_snapshots.is_empty() {
            return;
        }

        // Find variables that were newly defined in each branch
        let mut branch_new_vars: Vec<HashSet<String>> = branch_snapshots
            .iter()
            .map(|snap| snap.difference(before_snapshot).cloned().collect())
            .collect();

        // Variables defined in ALL branches become definitely defined
        if !branch_new_vars.is_empty() {
            let mut intersection: HashSet<String> = branch_new_vars[0].clone();
            for branch in branch_new_vars.iter().skip(1) {
                intersection = intersection.intersection(branch).cloned().collect();
            }
            for var in intersection {
                self.define(var);
            }
        }

        // Variables defined in SOME (but not all) branches become possibly defined
        let all_new_vars: HashSet<String> = branch_new_vars.drain(..).flatten().collect();
        for var in all_new_vars {
            if !self.is_defined(&var) {
                self.define_possibly(var);
            }
        }
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression, false);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::If(if_stmt) => {
                // Negative control flow: if (!$var) { return; } means $var is defined after
                let checked_vars = self.get_undefined_checked_vars(&if_stmt.condition);
                let body_exits = self.body_has_early_exit(&if_stmt.body);
                let has_early_exit = !checked_vars.is_empty() && body_exits;

                // Pattern: if (is_null($var)) { return; } means $var is defined after
                // is_null() on an undefined variable returns true, so if we pass this check,
                // the variable is defined (and not null)
                let is_null_checked_vars = self.get_is_null_checked_vars(&if_stmt.condition);
                let has_is_null_early_exit = !is_null_checked_vars.is_empty() && body_exits;

                // Pattern: if (!$var) { return; } means $var is defined (and truthy) after
                // This is similar to is_null but for general falsy checks
                let falsy_checked_vars = self.get_falsy_checked_vars(&if_stmt.condition);
                let has_falsy_early_exit = !falsy_checked_vars.is_empty() && body_exits;

                // Pattern: if (!isset($var)) { $var = ...; } means $var is defined after
                let isset_checked_vars = self.get_isset_checked_vars(&if_stmt.condition);
                let has_isset_pattern = !isset_checked_vars.is_empty();

                // Pattern: if (isset($var)) { use($var); } - $var is defined within the true branch
                let positive_isset_vars = self.get_positive_isset_vars(&if_stmt.condition);

                // Check for inverse condition pattern: if (!$cond || ...) where we saw if ($cond) before
                let inverse_base_cond = self.extract_inverse_base_condition(&if_stmt.condition);
                let previously_assigned_vars: HashSet<String> = if let Some(ref base_cond) = inverse_base_cond {
                    self.inverse_condition_assignments
                        .get(base_cond)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    HashSet::new()
                };

                // Get condition text for tracking
                let condition_text = self.normalize_condition(&self.get_condition_text(&if_stmt.condition));

                // For negative control flow with early exit, don't check the condition
                // because we know the variable will be defined after this point
                // Also skip for is_null() and falsy checks since they handle undefined variables
                if !has_early_exit && !has_is_null_early_exit && !has_falsy_early_exit {
                    self.analyze_expression(&if_stmt.condition, false);
                }

                // Check if there's an else clause
                let has_else = match &if_stmt.body {
                    IfBody::Statement(stmt_body) => stmt_body.else_clause.is_some(),
                    IfBody::ColonDelimited(block) => block.else_clause.is_some(),
                };

                // Check if there's an else or elseif clause (for falsy guard pattern)
                let has_else_or_elseif = match &if_stmt.body {
                    IfBody::Statement(stmt_body) => {
                        stmt_body.else_clause.is_some() || !stmt_body.else_if_clauses.is_empty()
                    }
                    IfBody::ColonDelimited(block) => {
                        block.else_clause.is_some() || !block.else_if_clauses.is_empty()
                    }
                };

                // Save state before analyzing if body (for inverse condition tracking)
                let before_if_state = self.current_scope().defined.clone();
                let before_if_possibly = self.current_scope().possibly_defined.clone();

                // ALWAYS mark positive isset vars as defined for if body analysis
                // This handles patterns like: if (isset($a) && !isset($b)) { use($a); }
                // The $a should be considered defined inside the if body
                for var in &positive_isset_vars {
                    self.current_scope_mut().defined.insert(var.clone());
                }

                // Push condition for correlation tracking
                self.push_condition(&if_stmt.condition);

                // If this is is_null() with early exit, handle similarly to isset pattern
                // Pattern: if (is_null($var)) { return; } -> $var is defined after
                if has_is_null_early_exit {
                    // Don't check the condition for undefined vars (is_null handles undefined)
                    // Just analyze the body for other errors, but don't merge branches
                    let before_state = self.current_scope().defined.clone();
                    let before_possibly = self.current_scope().possibly_defined.clone();

                    match &if_stmt.body {
                        IfBody::Statement(stmt_body) => {
                            self.analyze_statement(stmt_body.statement);
                        }
                        IfBody::ColonDelimited(block) => {
                            for inner in block.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }

                    // Restore the state (the exit branch doesn't contribute)
                    self.current_scope_mut().defined = before_state;
                    self.current_scope_mut().possibly_defined = before_possibly;

                    // Promote the is_null-checked variables to definitely defined
                    // If we pass is_null($var), then $var is defined (and not null)
                    for var in is_null_checked_vars {
                        if self.current_scope().is_possibly_defined(&var) {
                            self.current_scope_mut().possibly_defined.remove(&var);
                        }
                        self.define(var.clone());
                    }

                    // Pop condition and skip the rest of the if handling
                    self.pop_condition();
                    return;
                }

                // If this is falsy check (!$var) with early exit, handle similarly
                // Pattern: if (!$var) { return; } -> $var is defined (and truthy) after
                if has_falsy_early_exit {
                    // Don't check the condition for undefined vars (falsy check handles undefined)
                    let before_state = self.current_scope().defined.clone();
                    let before_possibly = self.current_scope().possibly_defined.clone();

                    match &if_stmt.body {
                        IfBody::Statement(stmt_body) => {
                            self.analyze_statement(stmt_body.statement);
                        }
                        IfBody::ColonDelimited(block) => {
                            for inner in block.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }

                    // Restore the state (the exit branch doesn't contribute)
                    self.current_scope_mut().defined = before_state;
                    self.current_scope_mut().possibly_defined = before_possibly;

                    // Promote the falsy-checked variables to definitely defined
                    // If we pass !$var check (meaning $var is truthy), then $var is defined
                    for var in falsy_checked_vars {
                        if self.current_scope().is_possibly_defined(&var) {
                            self.current_scope_mut().possibly_defined.remove(&var);
                        }
                        self.define(var.clone());
                    }

                    // Pop condition and skip the rest of the if handling
                    self.pop_condition();
                    return;
                }

                // If this is negative control flow with early exit, analyze normally but also
                // promote the checked variables after. The if body exits, so elseif/else branches
                // define variables that will be definitely defined after the if statement.
                if has_early_exit {
                    // Analyze the full if/elseif/else structure normally
                    // The analyze_if_body function already handles early exits by not including
                    // their snapshots in the merge
                    self.analyze_if_body(&if_stmt.body, has_else);

                    // Additionally promote the checked variables to definitely defined
                    // Pattern: if (!$var) { return; } means $var is defined after
                    for var in checked_vars {
                        if self.current_scope().is_possibly_defined(&var) {
                            self.current_scope_mut().possibly_defined.remove(&var);
                        }
                        self.define(var.clone());
                    }
                } else if has_isset_pattern && !has_else {
                    // Pattern: if (!isset($var)) { $var = ...; }
                    // After this if, $var is guaranteed to be defined

                    // Save the current state
                    let before_state = self.current_scope().defined.clone();

                    // Analyze the body
                    match &if_stmt.body {
                        IfBody::Statement(stmt_body) => {
                            self.analyze_statement(stmt_body.statement);
                        }
                        IfBody::ColonDelimited(block) => {
                            for inner in block.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }

                    // Get variables defined in the if block
                    let after_state = self.current_scope().defined.clone();

                    // For each isset-checked variable, if it was defined in the block, it's now guaranteed
                    for var in isset_checked_vars {
                        // Check if var was defined inside the block or was already defined
                        if after_state.contains(&var) || before_state.contains(&var) {
                            // After if (!isset($var)) { $var = X; }, $var is definitely defined
                            // because either:
                            // 1. It was already defined (isset was true, didn't enter block)
                            // 2. It wasn't defined, entered block, and got defined there
                            self.current_scope_mut().defined.insert(var);
                        }
                    }
                } else if !positive_isset_vars.is_empty() {
                    // Pattern: if (isset($var)) { use($var); }
                    // Within the true branch, variables checked by isset are guaranteed defined
                    self.analyze_if_body_with_isset_vars(&if_stmt.body, has_else, &positive_isset_vars);
                } else {
                    // Check for negated isset pattern: if (!isset($var)) { ... } else { use($var); }
                    // Variables checked by !isset are defined in the ELSE branch
                    let negated_isset_vars = self.get_isset_checked_vars(&if_stmt.condition);
                    if !negated_isset_vars.is_empty() && has_else {
                        self.analyze_if_body_with_negated_isset_vars(&if_stmt.body, &negated_isset_vars);
                    } else if !falsy_checked_vars.is_empty() && has_else_or_elseif {
                        // Pattern: if (!$var) { ... } elseif/else { use($var); }
                        // Variables checked by !$var are defined in the ELSE/ELSEIF branches
                        self.analyze_if_body_with_negated_falsy_vars(&if_stmt.body, &falsy_checked_vars);
                    } else {
                        // Normal if statement analysis
                        self.analyze_if_body(&if_stmt.body, has_else);
                    }
                }

                // Pop condition after analyzing if body
                self.pop_condition();

                // Track variables assigned in this if body for inverse condition detection
                // Include both definitely and possibly defined variables
                let after_if_defined = self.current_scope().defined.clone();
                let after_if_possibly = self.current_scope().possibly_defined.clone();
                let newly_defined = self.get_newly_assigned_vars(&before_if_state, &after_if_defined);
                let mut newly_assigned_in_if: HashSet<String> = newly_defined;
                // Also include newly possibly-defined variables (those assigned in if without else)
                for var in &after_if_possibly {
                    if !before_if_state.contains(var) && !before_if_possibly.contains(var) {
                        newly_assigned_in_if.insert(var.clone());
                    }
                }

                // Record the assignments under this condition for future inverse condition checks
                if !newly_assigned_in_if.is_empty() && !has_else {
                    self.record_inverse_condition_assignment(&condition_text, &newly_assigned_in_if);
                }

                // If this is an inverse condition pattern, promote variables that were assigned
                // in both the original condition and this inverse condition to definitely defined
                if !previously_assigned_vars.is_empty() {
                    for var in &previously_assigned_vars {
                        // If this var was assigned in BOTH the original if($cond) and this if(!$cond || ...)
                        // then it's guaranteed to be defined
                        // Check: was it assigned in this if body?
                        // - It's in newly_assigned_in_if (newly defined or possibly defined)
                        // - OR it was already possibly defined and this if also assigns it
                        // - OR it's already definitely defined
                        let was_reassigned_in_this_if = {
                            // The var is in after_if_defined or after_if_possibly
                            // and the if body contains an assignment to it
                            after_if_defined.contains(var) || after_if_possibly.contains(var)
                        };

                        if was_reassigned_in_this_if {
                            // Promote from possibly_defined to defined
                            self.current_scope_mut().possibly_defined.remove(var);
                            self.define(var.clone());
                        }
                    }
                }
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression, false);

                // Define the loop variables
                if let ForeachTarget::KeyValue(kv) = &foreach.target {
                    let key = self.get_var_name(&kv.key);
                    if let Some(name) = key {
                        self.define(name);
                    }
                    let value = self.get_var_name(&kv.value);
                    if let Some(name) = value {
                        self.define(name);
                    }
                } else if let ForeachTarget::Value(value) = &foreach.target {
                    let name = self.get_var_name(&value.value);
                    if let Some(n) = name {
                        self.define(n);
                    }
                }

                self.analyze_foreach_body(&foreach.body);
            }
            Statement::For(for_stmt) => {
                for expr in for_stmt.initializations.iter() {
                    self.analyze_expression(expr, false);
                }
                for expr in for_stmt.conditions.iter() {
                    self.analyze_expression(expr, false);
                }
                for expr in for_stmt.increments.iter() {
                    self.analyze_expression(expr, false);
                }
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_expression(&while_stmt.condition, false);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::DoWhile(do_while) => {
                self.analyze_statement(&do_while.statement);
                self.analyze_expression(&do_while.condition, false);
            }
            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    // Define the exception variable
                    if let Some(var) = &catch.variable {
                        let span = var.span();
                        let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                        self.define(name.to_string());
                    }
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
            Statement::Switch(switch) => {
                self.analyze_expression(&switch.expression, false);
                self.analyze_switch_body(&switch.body);
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.analyze_expression(expr, false);
                }
            }
            Statement::Echo(echo) => {
                for expr in echo.values.iter() {
                    self.analyze_expression(expr, false);
                }
            }
            Statement::Function(func) => {
                self.push_scope();
                // Define parameters
                for param in func.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }
                self.pop_scope();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    self.analyze_class_member(member);
                }
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.analyze_statement(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.analyze_statement(inner);
                        }
                    }
                }
            }
            Statement::Global(global) => {
                // Global statement defines the variable in current scope
                for var in global.variables.iter() {
                    let span = var.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
            }
            Statement::Static(static_stmt) => {
                // Static statement defines the variable in current scope
                for item in static_stmt.items.iter() {
                    let span = item.variable().span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
            }
            _ => {}
        }
    }

    fn analyze_class_member<'a>(&mut self, member: &ClassLikeMember<'a>) {
        if let ClassLikeMember::Method(method) = member {
            if let MethodBody::Concrete(body) = &method.body {
                // Use push_method_scope which sets has_this and defines $this
                self.push_method_scope();
                // Define parameters
                for param in method.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                for inner in body.statements.iter() {
                    self.analyze_statement(inner);
                }
                self.pop_scope();
            }
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>, has_else: bool) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        match body {
            IfBody::Statement(stmt_body) => {
                // Analyze 'if' branch
                self.analyze_statement(stmt_body.statement);
                let if_has_early_exit = self.statement_has_early_exit(stmt_body.statement);
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    self.analyze_statement(else_if.statement);
                    let elseif_has_early_exit = self.statement_has_early_exit(else_if.statement);
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch
                let else_has_early_exit = if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                    let has_exit = self.statement_has_early_exit(else_clause.statement);
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    has_exit
                } else {
                    false
                };

                // Special case: if one branch exits early and we have else
                // The non-exit branch's variables are definitely defined
                if has_else && if_has_early_exit && !else_has_early_exit && stmt_body.else_if_clauses.is_empty() {
                    // Only if branch exits, else branch defines variables that are definitely defined
                    // Don't merge as "possibly defined" - the else snapshot is already in branch_snapshots
                    // and we won't add the "no match" path, so it will be treated as definitely defined
                }
            }
            IfBody::ColonDelimited(block) => {
                // Analyze 'if' branch
                let mut if_has_early_exit = false;
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                    if self.statement_has_early_exit(inner) {
                        if_has_early_exit = true;
                    }
                }
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    let mut elseif_has_early_exit = false;
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            elseif_has_early_exit = true;
                        }
                    }
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch
                let else_has_early_exit = if let Some(else_clause) = &block.else_clause {
                    let mut has_exit = false;
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            has_exit = true;
                        }
                    }
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    has_exit
                } else {
                    false
                };

                // Special case: if one branch exits early and we have else
                if has_else && if_has_early_exit && !else_has_early_exit && block.else_if_clauses.is_empty() {
                    // Only if branch exits, else branch defines variables that are definitely defined
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        // If there's no else clause, we need to consider the "fall-through" path
        // where the if condition was false and no branch was taken
        if !has_else {
            // Add the "no branch taken" snapshot (same as before_snapshot)
            branch_snapshots.push(before_snapshot.clone());
        }

        self.merge_branches(&before_snapshot, branch_snapshots);
    }

    /// Analyze if body where the condition contains positive isset() checks.
    /// Variables checked by isset() are marked as defined within the true branch.
    fn analyze_if_body_with_isset_vars<'a>(
        &mut self,
        body: &IfBody<'a>,
        has_else: bool,
        isset_vars: &[String],
    ) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        // Mark isset variables as temporarily defined for the true branch
        for var in isset_vars {
            self.define(var.clone());
        }

        match body {
            IfBody::Statement(stmt_body) => {
                // Analyze 'if' branch (with isset vars defined)
                self.analyze_statement(stmt_body.statement);
                let if_has_early_exit = self.statement_has_early_exit(stmt_body.statement);
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches (isset vars NOT guaranteed here)
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    self.analyze_statement(else_if.statement);
                    let elseif_has_early_exit = self.statement_has_early_exit(else_if.statement);
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch (isset vars NOT guaranteed here)
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                    let has_exit = self.statement_has_early_exit(else_clause.statement);
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
            IfBody::ColonDelimited(block) => {
                // Analyze 'if' branch (with isset vars defined)
                let mut if_has_early_exit = false;
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                    if self.statement_has_early_exit(inner) {
                        if_has_early_exit = true;
                    }
                }
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    let mut elseif_has_early_exit = false;
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            elseif_has_early_exit = true;
                        }
                    }
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch
                if let Some(else_clause) = &block.else_clause {
                    let mut has_exit = false;
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            has_exit = true;
                        }
                    }
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        // If there's no else clause, we need to consider the "fall-through" path
        if !has_else {
            branch_snapshots.push(before_snapshot.clone());
        }

        self.merge_branches(&before_snapshot, branch_snapshots);
    }

    /// Analyze if body where the condition is !isset($var).
    /// Variables checked by !isset() are marked as defined within the ELSE branch.
    /// Pattern: if (!isset($var)) { ... } else { use($var); }
    fn analyze_if_body_with_negated_isset_vars<'a>(
        &mut self,
        body: &IfBody<'a>,
        negated_isset_vars: &[String],
    ) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        match body {
            IfBody::Statement(stmt_body) => {
                // Analyze 'if' branch (negated isset vars NOT defined here)
                self.analyze_statement(stmt_body.statement);
                let if_has_early_exit = self.statement_has_early_exit(stmt_body.statement);
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for else branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'else' branch (negated isset vars ARE defined here)
                if let Some(else_clause) = &stmt_body.else_clause {
                    // Mark negated isset vars as defined for else branch
                    for var in negated_isset_vars {
                        self.define(var.clone());
                    }

                    self.analyze_statement(else_clause.statement);
                    let has_exit = self.statement_has_early_exit(else_clause.statement);
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
            IfBody::ColonDelimited(block) => {
                // Analyze 'if' branch (negated isset vars NOT defined here)
                let mut if_has_early_exit = false;
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                    if self.statement_has_early_exit(inner) {
                        if_has_early_exit = true;
                    }
                }
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for else branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'else' branch (negated isset vars ARE defined here)
                if let Some(else_clause) = &block.else_clause {
                    // Mark negated isset vars as defined for else branch
                    for var in negated_isset_vars {
                        self.define(var.clone());
                    }

                    let mut has_exit = false;
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            has_exit = true;
                        }
                    }
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        self.merge_branches(&before_snapshot, branch_snapshots);
    }

    /// Analyze if body where the condition is !$var
    /// Pattern: if (!$var) { ... } else { use($var); }
    /// The falsy-checked variables are defined in ELSE/ELSEIF branches
    fn analyze_if_body_with_negated_falsy_vars<'a>(
        &mut self,
        body: &IfBody<'a>,
        falsy_vars: &[String],
    ) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        match body {
            IfBody::Statement(stmt_body) => {
                // Analyze 'if' branch (falsy vars NOT defined here - var is falsy/undefined)
                self.analyze_statement(stmt_body.statement);
                let if_has_early_exit = self.statement_has_early_exit(stmt_body.statement);
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for else branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches (falsy vars ARE defined here - !$var was false)
                for else_if in stmt_body.else_if_clauses.iter() {
                    // Mark falsy vars as defined for elseif branch
                    for var in falsy_vars {
                        self.define(var.clone());
                    }

                    self.analyze_expression(&else_if.condition, false);
                    self.analyze_statement(else_if.statement);
                    let elseif_has_early_exit = self.statement_has_early_exit(else_if.statement);
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch (falsy vars ARE defined here)
                if let Some(else_clause) = &stmt_body.else_clause {
                    // Mark falsy vars as defined for else branch
                    for var in falsy_vars {
                        self.define(var.clone());
                    }

                    self.analyze_statement(else_clause.statement);
                    let has_exit = self.statement_has_early_exit(else_clause.statement);
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
            IfBody::ColonDelimited(block) => {
                // Analyze 'if' branch (falsy vars NOT defined here)
                let mut if_has_early_exit = false;
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                    if self.statement_has_early_exit(inner) {
                        if_has_early_exit = true;
                    }
                }
                if !if_has_early_exit {
                    branch_snapshots.push(self.current_scope().snapshot());
                }

                // Reset to before state for else branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches (falsy vars ARE defined here)
                for else_if in block.else_if_clauses.iter() {
                    // Mark falsy vars as defined for elseif branch
                    for var in falsy_vars {
                        self.define(var.clone());
                    }

                    self.analyze_expression(&else_if.condition, false);
                    let mut elseif_has_early_exit = false;
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            elseif_has_early_exit = true;
                        }
                    }
                    if !elseif_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch (falsy vars ARE defined here)
                if let Some(else_clause) = &block.else_clause {
                    // Mark falsy vars as defined for else branch
                    for var in falsy_vars {
                        self.define(var.clone());
                    }

                    let mut has_exit = false;
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                        if self.statement_has_early_exit(inner) {
                            has_exit = true;
                        }
                    }
                    if !has_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        self.merge_branches(&before_snapshot, branch_snapshots);
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_switch_body<'a>(&mut self, body: &SwitchBody<'a>) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        let has_default = match body {
            SwitchBody::BraceDelimited(block) => {
                let has_default_case = self.switch_has_default(&block.cases);
                for case in block.cases.iter() {
                    // Analyze this case as a separate branch
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }

                    // Check if case has early exit
                    let case_has_early_exit = self.case_has_early_exit(case);

                    // Only save snapshot if case has statements AND doesn't have early exit
                    // Skip empty fallthrough cases and cases that return/throw/exit
                    if !case.statements().is_empty() && !case_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }

                    // Reset to before state for next case
                    // (PHP switch has fallthrough, but with early returns each case is independent)
                    self.current_scope_mut().defined = before_snapshot.clone();
                }
                has_default_case
            }
            SwitchBody::ColonDelimited(block) => {
                let has_default_case = self.switch_has_default(&block.cases);
                for case in block.cases.iter() {
                    // Analyze this case as a separate branch
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }

                    // Check if case has early exit
                    let case_has_early_exit = self.case_has_early_exit(case);

                    // Only save snapshot if case has statements AND doesn't have early exit
                    // Skip empty fallthrough cases and cases that return/throw/exit
                    if !case.statements().is_empty() && !case_has_early_exit {
                        branch_snapshots.push(self.current_scope().snapshot());
                    }

                    // Reset to before state for next case
                    self.current_scope_mut().defined = before_snapshot.clone();
                }
                has_default_case
            }
        };

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        // If there's no default case, we need to consider the "no match" path
        // where none of the cases matched and execution continues after the switch
        if !has_default {
            // Add the "no match" snapshot (same as before_snapshot)
            branch_snapshots.push(before_snapshot.clone());
        }

        self.merge_branches(&before_snapshot, branch_snapshots);
    }

    fn switch_has_default<'a>(&self, cases: &Sequence<'a, SwitchCase<'a>>) -> bool {
        cases.iter().any(|case| matches!(case, SwitchCase::Default(_)))
    }

    /// Check if a switch case has an early exit (return, throw, exit, die)
    /// A case exits early if any statement in it is an early exit
    fn case_has_early_exit<'a>(&self, case: &SwitchCase<'a>) -> bool {
        let statements = case.statements();
        for stmt in statements.iter() {
            if self.statement_has_early_exit(stmt) {
                return true;
            }
        }
        false
    }

    fn get_var_name<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Variable(var) => {
                let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];
                Some(name.to_string())
            }
            // Handle reference expressions: &$var (represented as UnaryPrefix with Reference operator)
            Expression::UnaryPrefix(unary) => {
                if matches!(unary.operator, UnaryPrefixOperator::Reference(_)) {
                    self.get_var_name(&unary.operand)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract variables being checked with isset pattern: !isset($var)
    /// This is used for pattern: if (!isset($var)) { $var = ...; }
    fn get_isset_checked_vars<'a>(&self, condition: &Expression<'a>) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_isset_checked_vars(condition, &mut vars);
        vars
    }

    fn extract_isset_checked_vars<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>) {
        match expr {
            // Pattern: !isset($var)
            Expression::UnaryPrefix(unary) => {
                if let Expression::Construct(Construct::Isset(isset)) = &*unary.operand {
                    for value in isset.values.iter() {
                        if let Some(var_name) = self.get_var_name(value) {
                            vars.push(var_name);
                        }
                    }
                }
            }
            // Pattern: !isset($a) || !isset($b)
            Expression::Binary(binary) => {
                self.extract_isset_checked_vars(&binary.lhs, vars);
                self.extract_isset_checked_vars(&binary.rhs, vars);
            }
            _ => {}
        }
    }

    /// Extract variables from POSITIVE isset/!empty conditions: isset($var), isset($var['key']), !empty($var)
    /// This is used for pattern: if (isset($adminRow['key'])) { use($adminRow); }
    /// or: if (!empty($var)) { use($var); }
    /// Within the true branch, $var is guaranteed to be defined.
    fn get_positive_isset_vars<'a>(&self, condition: &Expression<'a>) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_positive_isset_vars(condition, &mut vars, false);
        vars
    }

    fn extract_positive_isset_vars<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>, negated: bool) {
        match expr {
            // Direct isset($var) or isset($var['key']) or isset($var->prop)
            Expression::Construct(Construct::Isset(isset)) => {
                if !negated {
                    for value in isset.values.iter() {
                        self.extract_base_variable(value, vars);
                    }
                }
            }
            // empty($var) - if negated (i.e., !empty($var)), the variable is defined
            Expression::Construct(Construct::Empty(empty)) => {
                if negated {
                    // !empty($var) means $var is defined and truthy
                    self.extract_base_variable(empty.value, vars);
                }
            }
            // !isset($var) or !empty($var) - flip the negation
            Expression::UnaryPrefix(unary) => {
                // Check if this is a negation operator
                let span = unary.operator.span();
                let op = &self.source[span.start.offset as usize..span.end.offset as usize];
                if op == "!" {
                    self.extract_positive_isset_vars(&unary.operand, vars, !negated);
                }
            }
            // isset($a) && isset($b) - both must be true, include all
            Expression::Binary(binary) => {
                // For AND conditions with isset, both sides contribute
                let op_span = binary.operator.span();
                let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];
                if op == "&&" && !negated {
                    self.extract_positive_isset_vars(&binary.lhs, vars, negated);
                    self.extract_positive_isset_vars(&binary.rhs, vars, negated);
                }
                // For OR conditions, we can't guarantee either side
            }
            // Parenthesized expression
            Expression::Parenthesized(paren) => {
                self.extract_positive_isset_vars(&paren.expression, vars, negated);
            }
            _ => {}
        }
    }

    /// Extract the base variable from an expression like $var, $var['key'], $var->prop
    fn extract_base_variable<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>) {
        match expr {
            Expression::Variable(var) => {
                let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];
                if !vars.contains(&name.to_string()) {
                    vars.push(name.to_string());
                }
            }
            // $var['key'] - extract $var
            Expression::ArrayAccess(access) => {
                self.extract_base_variable(&access.array, vars);
            }
            // $var->prop - extract $var
            Expression::Access(Access::Property(access)) => {
                self.extract_base_variable(&access.object, vars);
            }
            // $var?->prop - extract $var
            Expression::Access(Access::NullSafeProperty(access)) => {
                self.extract_base_variable(&access.object, vars);
            }
            _ => {}
        }
    }

    /// Define all variables in a list() expression
    /// Handles: list($a, $b) = ..., list($a, list($b, $c)) = ..., list('key' => $a) = ...
    fn define_list_variables<'a>(&mut self, list: &mago_syntax::ast::List<'a>) {
        for elem in list.elements.iter() {
            match elem {
                ArrayElement::Value(val) => {
                    // Can be a variable or nested list
                    self.define_list_element(&val.value);
                }
                ArrayElement::KeyValue(kv) => {
                    // list('key' => $var) - the key is not a definition, only the value is
                    self.define_list_element(&kv.value);
                }
                ArrayElement::Missing(_) => {
                    // list(, $b) - skipped element
                }
                _ => {}
            }
        }
    }

    /// Define a variable from a list element (can be variable, nested list, or array access)
    fn define_list_element<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::Variable(var) => {
                let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];
                self.define(name.to_string());
            }
            Expression::List(nested_list) => {
                // Nested list: list($a, list($b, $c)) = ...
                self.define_list_variables(nested_list);
            }
            Expression::Array(nested_arr) => {
                // Nested array destructuring: [$a, [$b, $c]] = ...
                self.define_array_destructuring_variables(nested_arr);
            }
            Expression::ArrayAccess(access) => {
                // list($arr['key']) = ... - defines $arr (or extends it)
                // Just analyze as assignment LHS
                self.analyze_expression(expr, true);
            }
            _ => {
                // For any other LHS expression, treat as assignment target
                self.analyze_expression(expr, true);
            }
        }
    }

    /// Define all variables in a short array destructuring: [$a, $b] = ...
    fn define_array_destructuring_variables<'a>(&mut self, arr: &mago_syntax::ast::Array<'a>) {
        for elem in arr.elements.iter() {
            match elem {
                ArrayElement::Value(val) => {
                    // Can be a variable, nested array, or nested list
                    self.define_list_element(&val.value);
                }
                ArrayElement::KeyValue(kv) => {
                    // ['key' => $var] - the key is not a definition, only the value is
                    self.define_list_element(&kv.value);
                }
                ArrayElement::Missing(_) => {
                    // [, $b] - skipped element
                }
                _ => {}
            }
        }
    }

    /// Extract variables checked by !$var (falsy check) in a condition
    /// Pattern: if (!$var) { return; } -> after this, $var is defined and truthy
    fn get_falsy_checked_vars<'a>(&self, condition: &Expression<'a>) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_falsy_checked_vars(condition, &mut vars);
        vars
    }

    fn extract_falsy_checked_vars<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>) {
        match expr {
            // Pattern: !$var
            Expression::UnaryPrefix(unary) => {
                let op_span = unary.operator.span();
                let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];
                if op == "!" {
                    // Check if operand is a simple variable
                    if let Some(var_name) = self.get_var_name(&unary.operand) {
                        vars.push(var_name);
                    }
                }
            }
            // Pattern: !$a || !$b
            Expression::Binary(binary) => {
                let op_span = binary.operator.span();
                let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];
                if op == "||" || op == "or" {
                    self.extract_falsy_checked_vars(&binary.lhs, vars);
                    self.extract_falsy_checked_vars(&binary.rhs, vars);
                }
            }
            Expression::Parenthesized(paren) => {
                self.extract_falsy_checked_vars(&paren.expression, vars);
            }
            _ => {}
        }
    }

    /// Extract variables checked by is_null($var) in a condition
    /// Pattern: if (is_null($var)) { return; } -> after this, $var is defined
    fn get_is_null_checked_vars<'a>(&self, condition: &Expression<'a>) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_is_null_checked_vars(condition, &mut vars);
        vars
    }

    fn extract_is_null_checked_vars<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>) {
        match expr {
            // Pattern: is_null($var)
            Expression::Call(Call::Function(call)) => {
                let func_name = match &*call.function {
                    Expression::Identifier(ident) => {
                        self.get_span_text(&ident.span())
                    }
                    _ => return,
                };
                if func_name == "is_null" && !call.argument_list.arguments.is_empty() {
                    if let Some(arg) = call.argument_list.arguments.first() {
                        if let Some(var_name) = self.get_var_name(arg.value()) {
                            vars.push(var_name);
                        }
                    }
                }
            }
            // Pattern: is_null($a) || is_null($b)
            Expression::Binary(binary) => {
                let op_span = binary.operator.span();
                let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];
                if op == "||" || op == "or" {
                    self.extract_is_null_checked_vars(&binary.lhs, vars);
                    self.extract_is_null_checked_vars(&binary.rhs, vars);
                }
            }
            Expression::Parenthesized(paren) => {
                self.extract_is_null_checked_vars(&paren.expression, vars);
            }
            _ => {}
        }
    }

    /// Extract variables being checked for undefined/empty/false in a condition
    /// Returns variables from patterns like: !$var, empty($var), !isset($var), !$var || !$other
    fn get_undefined_checked_vars<'a>(&self, condition: &Expression<'a>) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_undefined_checked_vars(condition, &mut vars);
        vars
    }

    fn extract_undefined_checked_vars<'a>(&self, expr: &Expression<'a>, vars: &mut Vec<String>) {
        match expr {
            // Pattern: !$var or !isset($var)
            Expression::UnaryPrefix(unary) => {
                // Check if it's !isset($var) - isset is a language construct
                if let Expression::Construct(Construct::Isset(isset)) = &*unary.operand {
                    for value in isset.values.iter() {
                        if let Some(var_name) = self.get_var_name(value) {
                            vars.push(var_name);
                        }
                    }
                    return;
                }

                // Check if it's !empty($var) - though this is less common
                if let Expression::Construct(Construct::Empty(empty)) = &*unary.operand {
                    if let Some(var_name) = self.get_var_name(empty.value) {
                        vars.push(var_name);
                    }
                    return;
                }

                // Pattern: !$var
                if let Some(var_name) = self.get_var_name(&unary.operand) {
                    vars.push(var_name);
                }
            }
            // Pattern: empty($var) - language construct
            Expression::Construct(Construct::Empty(empty)) => {
                if let Some(var_name) = self.get_var_name(empty.value) {
                    vars.push(var_name);
                }
            }
            // Pattern: !$a || !$b (multiple checks with OR)
            Expression::Binary(binary) => {
                // For OR conditions, both sides are checked
                self.extract_undefined_checked_vars(&binary.lhs, vars);
                self.extract_undefined_checked_vars(&binary.rhs, vars);
            }
            _ => {}
        }
    }

    /// Check if a body has an early exit (return, exit, throw, die)
    fn body_has_early_exit<'a>(&self, body: &IfBody<'a>) -> bool {
        match body {
            IfBody::Statement(stmt_body) => {
                self.statement_has_early_exit(stmt_body.statement)
            }
            IfBody::ColonDelimited(block) => {
                // Check if any statement in the block is an early exit
                for stmt in block.statements.iter() {
                    if self.statement_has_early_exit(stmt) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn statement_has_early_exit<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            Statement::Return(_) => true,
            Statement::Block(block) => {
                // Check if any statement in the block is an early exit
                for inner in block.statements.iter() {
                    if self.statement_has_early_exit(inner) {
                        return true;
                    }
                }
                false
            }
            Statement::Expression(expr_stmt) => {
                // Check for throw expressions
                if matches!(&expr_stmt.expression, Expression::Throw(_)) {
                    return true;
                }

                // Check for exit and die as language constructs
                if let Expression::Construct(construct) = &expr_stmt.expression {
                    if matches!(construct, Construct::Exit(_) | Construct::Die(_)) {
                        return true;
                    }
                }

                // Check for exit() or die() calls (fallback, though they should be constructs)
                if let Expression::Call(Call::Function(call)) = &expr_stmt.expression {
                    let func_name = match &*call.function {
                        Expression::Identifier(ident) => {
                            self.get_span_text(&ident.span())
                        }
                        _ => return false,
                    };
                    return func_name == "exit" || func_name == "die";
                }

                // Check for static method calls that exit (e.g., Result::setErrorAndExit())
                if let Expression::Call(Call::StaticMethod(call)) = &expr_stmt.expression {
                    let method_name = self.get_span_text(&call.method.span());
                    // Methods ending with "AndExit" or "Exit" are considered exits
                    if method_name.ends_with("AndExit") || method_name.ends_with("Exit") {
                        return true;
                    }
                }

                // Check for instance method calls that exit (e.g., $this->exitWithError())
                if let Expression::Call(Call::Method(call)) = &expr_stmt.expression {
                    let method_name = self.get_span_text(&call.method.span());
                    // Methods ending with "AndExit" or "Exit" are considered exits
                    if method_name.ends_with("AndExit") || method_name.ends_with("Exit") {
                        return true;
                    }
                }

                false
            }
            _ => false,
        }
    }

    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>, is_assignment_lhs: bool) {
        match expr {
            Expression::Variable(var) => {
                let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];

                // If this is the left side of an assignment, it defines the variable
                if is_assignment_lhs {
                    self.define(name.to_string());
                    // Record this assignment under current conditions for correlation tracking
                    self.record_conditional_assignment(name);
                } else if !name.starts_with("$$") {
                    // Check if it's defined or possibly defined
                    // IMPORTANT: Check is_defined FIRST to avoid false positives when a variable
                    // is definitely defined in the current scope but possibly defined in an outer scope
                    if !self.is_defined(name) {
                        // Also check condition correlation: if the variable was assigned under the same
                        // condition we're currently in, it's guaranteed to be defined
                        if self.is_defined_by_condition_correlation(name) {
                            // Variable is defined via condition correlation - no error
                        } else if self.is_possibly_defined(name) {
                            // Variable is defined in some branches but not all
                            let (line, col) = self.get_line_col(var.span().start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "undefined.variable",
                                    format!("Variable {} might not be defined.", name),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("variable.possiblyUndefined"),
                            );
                        } else {
                            // Variable is completely undefined
                            let (line, col) = self.get_line_col(var.span().start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "undefined.variable",
                                    format!("Undefined variable {}", name),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("variable.undefined"),
                            );
                        }
                    }
                }
            }
            Expression::Assignment(assign) => {
                // First analyze RHS (before LHS is defined)
                self.analyze_expression(&assign.rhs, false);
                // Then analyze LHS as assignment target
                // Handle list() assignments: list($a, $b) = $array
                if let Expression::List(list) = &*assign.lhs {
                    self.define_list_variables(list);
                } else if let Expression::Array(arr) = &*assign.lhs {
                    // Handle short array destructuring: [$a, $b] = $array (PHP 7.1+)
                    self.define_array_destructuring_variables(arr);
                } else {
                    self.analyze_expression(&assign.lhs, true);
                }
            }
            Expression::List(list) => {
                // When list() appears in non-assignment context, analyze its elements
                for elem in list.elements.iter() {
                    match elem {
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value, is_assignment_lhs);
                        }
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key, false);
                            self.analyze_expression(&kv.value, is_assignment_lhs);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Closure(closure) => {
                self.push_closure_scope();

                // Add parameters to closure scope
                for param in closure.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }

                // Add `use` variables to closure scope
                if let Some(use_clause) = &closure.use_clause {
                    for var in use_clause.variables.iter() {
                        let span = var.variable.span();
                        let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                        self.current_scope_mut().inherited.insert(name.to_string());
                    }
                }

                // Analyze closure body
                for stmt in closure.body.statements.iter() {
                    self.analyze_statement(stmt);
                }

                self.pop_scope();
            }
            Expression::ArrowFunction(arrow) => {
                // Arrow functions inherit parent scope
                // Add parameters
                for param in arrow.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                // Analyze expression
                self.analyze_expression(arrow.expression, false);
            }
            Expression::Call(Call::Function(call)) => {
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Call(Call::Method(call)) => {
                self.analyze_expression(&call.object, false);
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Call(Call::StaticMethod(call)) => {
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Binary(binary) => {
                // Special handling for null coalesce operator: $var ?? "default"
                // The LHS is allowed to be undefined/null - that's the whole point
                if matches!(binary.operator, BinaryOperator::NullCoalesce(_)) {
                    // Skip checking LHS for undefined, only analyze RHS
                    self.analyze_expression(&binary.rhs, false);
                } else if matches!(binary.operator, BinaryOperator::And(_) | BinaryOperator::LowAnd(_)) {
                    // Short-circuit && evaluation: if LHS has isset($var), $var is defined for RHS
                    // Pattern: isset($var['key']) && $var['key'] == 'value'
                    let isset_vars = self.get_positive_isset_vars(&binary.lhs);

                    // Analyze LHS first
                    self.analyze_expression(&binary.lhs, false);

                    // Mark isset vars as defined for RHS analysis
                    if !isset_vars.is_empty() {
                        let before_defined = self.current_scope().defined.clone();
                        for var in &isset_vars {
                            self.current_scope_mut().defined.insert(var.clone());
                        }

                        // Analyze RHS with isset vars defined
                        self.analyze_expression(&binary.rhs, false);

                        // Restore original defined state (isset only guarantees for RHS, not after)
                        self.current_scope_mut().defined = before_defined;
                    } else {
                        self.analyze_expression(&binary.rhs, false);
                    }
                } else if matches!(binary.operator, BinaryOperator::Or(_) | BinaryOperator::LowOr(_)) {
                    // Short-circuit || evaluation: if LHS is !$cond and $cond previously assigned $var,
                    // then $var is defined for RHS evaluation
                    // Pattern: !$resending || $documentText === 'not found'
                    // If $resending was true, $documentText was defined, and RHS is evaluated with it defined

                    // Check if LHS is a negated condition that we've seen before
                    let inverse_base = self.extract_negated_condition(&binary.lhs);
                    let vars_from_inverse: HashSet<String> = if let Some(ref base_cond) = inverse_base {
                        self.inverse_condition_assignments
                            .get(base_cond)
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        HashSet::new()
                    };

                    // Analyze LHS first
                    self.analyze_expression(&binary.lhs, false);

                    // If we have inverse condition vars, mark them as defined for RHS
                    if !vars_from_inverse.is_empty() {
                        let before_defined = self.current_scope().defined.clone();
                        for var in &vars_from_inverse {
                            self.current_scope_mut().defined.insert(var.clone());
                        }

                        // Analyze RHS with inverse condition vars defined
                        self.analyze_expression(&binary.rhs, false);

                        // Restore original defined state
                        self.current_scope_mut().defined = before_defined;
                    } else {
                        self.analyze_expression(&binary.rhs, false);
                    }
                } else {
                    self.analyze_expression(&binary.lhs, false);
                    self.analyze_expression(&binary.rhs, false);
                }
            }
            Expression::Conditional(ternary) => {
                // Check if condition has isset() - variables are defined in the "then" branch
                let isset_vars = self.get_positive_isset_vars(&ternary.condition);

                // Check if condition is a simple variable that guards itself
                // Pattern: $var ? $var : default - the variable guards its own usage
                let condition_var = self.get_var_name(&ternary.condition);
                let is_self_guarding = if let Some(ref cond_var) = condition_var {
                    if let Some(then) = &ternary.then {
                        // Check if then branch uses the same variable
                        self.get_var_name(then) == Some(cond_var.clone())
                    } else {
                        // Elvis operator: $var ?: default
                        true
                    }
                } else {
                    false
                };

                // Check for condition correlation: if the ternary condition matches
                // a previously-seen if condition, variables defined under that condition
                // are also defined in the ternary's "then" branch
                // Pattern: if ($cond) { $var = X; } ... $cond ? use($var) : null
                let ternary_cond_text = self.normalize_condition(&self.get_condition_text(&ternary.condition));
                let correlated_vars: Vec<String> = self.condition_assignments
                    .get(&ternary_cond_text)
                    .map(|vars| vars.iter().cloned().collect())
                    .unwrap_or_default();

                // If self-guarding, don't report the condition variable as undefined
                // (the ternary handles undefined like isset does)
                if is_self_guarding {
                    // Skip analyzing the condition for this specific variable
                    // The truthy check guards the usage
                } else {
                    self.analyze_expression(&ternary.condition, false);
                }

                if let Some(then) = &ternary.then {
                    if !isset_vars.is_empty() || !correlated_vars.is_empty() {
                        // Mark isset vars and correlated vars as defined for "then" branch analysis
                        let before_defined = self.current_scope().defined.clone();
                        for var in &isset_vars {
                            self.current_scope_mut().defined.insert(var.clone());
                        }
                        for var in &correlated_vars {
                            self.current_scope_mut().defined.insert(var.clone());
                        }

                        self.analyze_expression(then, false);

                        // Restore original defined state
                        self.current_scope_mut().defined = before_defined;
                    } else if is_self_guarding {
                        // The condition variable is defined in the "then" branch
                        // because we only reach here if the condition was truthy
                        let before_defined = self.current_scope().defined.clone();
                        if let Some(ref cond_var) = condition_var {
                            self.current_scope_mut().defined.insert(cond_var.clone());
                        }

                        self.analyze_expression(then, false);

                        // Restore original defined state
                        self.current_scope_mut().defined = before_defined;
                    } else {
                        self.analyze_expression(then, false);
                    }
                }
                self.analyze_expression(&ternary.r#else, false);
            }
            Expression::Parenthesized(paren) => {
                self.analyze_expression(&paren.expression, false);
            }
            Expression::UnaryPrefix(unary) => {
                self.analyze_expression(&unary.operand, false);
            }
            Expression::UnaryPostfix(unary) => {
                self.analyze_expression(&unary.operand, false);
            }
            Expression::ArrayAccess(access) => {
                self.analyze_expression(&access.array, is_assignment_lhs);
                self.analyze_expression(&access.index, false);
            }
            Expression::Access(Access::Property(access)) => {
                self.analyze_expression(&access.object, false);
            }
            Expression::Access(Access::NullSafeProperty(access)) => {
                self.analyze_expression(&access.object, false);
            }
            Expression::Array(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key, false);
                            self.analyze_expression(&kv.value, false);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value, false);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value, false);
                        }
                        _ => {}
                    }
                }
            }
            Expression::LegacyArray(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key, false);
                            self.analyze_expression(&kv.value, false);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value, false);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value, false);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Instantiation(instantiate) => {
                if let Some(arg_list) = &instantiate.argument_list {
                    for arg in arg_list.arguments.iter() {
                        self.analyze_expression(arg.value(), false);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_operations() {
        let mut scope = Scope::new();
        scope.define("$foo".to_string());
        assert!(scope.is_defined("$foo"));
        assert!(!scope.is_defined("$bar"));
    }
}
