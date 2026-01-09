//! Expression type resolver
//!
//! Resolves the type of PHP expressions based on scope and symbol table.

use crate::scope::Scope;
use crate::symbols::SymbolTable;
use crate::types::Type;
use crate::types::phpdoc::parse_type_string;
use mago_syntax::ast::*;

/// Resolves expression types
pub struct ExpressionResolver<'a> {
    symbol_table: &'a SymbolTable,
    source: &'a str,
}

impl<'a> ExpressionResolver<'a> {
    /// Create a new expression resolver
    pub fn new(symbol_table: &'a SymbolTable, source: &'a str) -> Self {
        Self { symbol_table, source }
    }

    /// Resolve the type of an expression
    pub fn resolve(&self, expr: &Expression<'_>, scope: &Scope) -> Type {
        match expr {
            Expression::Literal(lit) => self.resolve_literal(lit),
            Expression::Variable(var) => self.resolve_variable(var, scope),
            Expression::Array(arr) => self.resolve_array(arr, scope),
            Expression::LegacyArray(arr) => self.resolve_legacy_array(arr, scope),
            Expression::List(_) => Type::mixed_array(),
            Expression::ArrayAccess(access) => self.resolve_array_access(access, scope),
            Expression::ArrayAppend(_) => Type::Mixed, // Array append doesn't have a value
            Expression::Parenthesized(paren) => self.resolve(&paren.expression, scope),
            Expression::Closure(closure) => self.resolve_closure(closure),
            Expression::ArrowFunction(_) => Type::Closure,
            Expression::New(new) => self.resolve_new(new, scope),
            Expression::Clone(clone) => self.resolve(&clone.object, scope),
            Expression::ObjectOperation(op) => self.resolve_object_operation(op, scope),
            Expression::ClassOperation(op) => self.resolve_class_operation(op, scope),
            Expression::Call(call) => self.resolve_call(call, scope),
            Expression::ClosureCreation(_) => Type::Closure,
            Expression::Ternary(ternary) => self.resolve_ternary(ternary, scope),
            Expression::Coalesce(coalesce) => self.resolve_coalesce(coalesce, scope),
            Expression::CoalesceAssignment(_) => Type::Mixed,
            Expression::Match(m) => self.resolve_match(m, scope),
            Expression::Yield(_) | Expression::YieldFrom(_) => Type::Mixed,
            Expression::Throw(_) => Type::Never,
            Expression::Instanceof(_) => Type::Bool,
            Expression::Reference(r) => self.resolve(&r.expression, scope),
            Expression::Suppressed(s) => self.resolve(&s.expression, scope),
            Expression::Binary(binary) => self.resolve_binary(binary, scope),
            Expression::Unary(unary) => self.resolve_unary(unary, scope),
            Expression::AssignmentOperation(assign) => self.resolve(&assign.rhs, scope),
            Expression::MagicConstant(mc) => self.resolve_magic_constant(mc),
            Expression::Identifier(_) | Expression::Name(_) => Type::Mixed,
            Expression::Static(_) => Type::Static,
            Expression::Self_(_) => Type::SelfType,
            Expression::Parent(_) => Type::Parent,
            Expression::Include(_) | Expression::Require(_) => Type::Mixed,
            Expression::Cast(cast) => self.resolve_cast(cast),
            Expression::Eval(_) => Type::Mixed,
            Expression::Empty(_) | Expression::Isset(_) | Expression::Unset(_) | Expression::Print(_) | Expression::Exit(_) => Type::Bool,
            _ => Type::Mixed,
        }
    }

    /// Resolve literal expression type
    fn resolve_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Null(_) => Type::Null,
            Literal::False(_) => Type::ConstantBool(false),
            Literal::True(_) => Type::ConstantBool(true),
            Literal::Integer(i) => {
                // Try to parse the integer value
                let text = self.get_span_text(&i.token.span);
                if let Ok(val) = text.parse::<i64>() {
                    Type::ConstantInt(val)
                } else {
                    Type::Int
                }
            }
            Literal::Float(_) => Type::Float,
            Literal::String(s) => {
                let text = self.get_span_text(&s.value.span);
                // Remove quotes
                let content = text.trim_matches(|c| c == '"' || c == '\'');
                if content.len() < 100 {
                    // Only store small strings as constants
                    Type::ConstantString(content.to_string())
                } else {
                    Type::String
                }
            }
            Literal::CompositeString(_) => Type::String,
        }
    }

    /// Resolve variable type
    fn resolve_variable(&self, var: &Variable, scope: &Scope) -> Type {
        match var {
            Variable::Direct(direct) => {
                let name = self.get_span_text(&direct.name.span);
                // Remove $ prefix
                let name = name.trim_start_matches('$');
                scope.get_variable_type(name).unwrap_or(Type::Mixed)
            }
            Variable::Indirect(_) => Type::Mixed, // $$var
            Variable::Nested(_) => Type::Mixed,   // $a->b
        }
    }

    /// Resolve array literal type
    fn resolve_array(&self, arr: &ArrayExpression, scope: &Scope) -> Type {
        if arr.elements.is_empty() {
            return Type::Array {
                key: Box::new(Type::Mixed),
                value: Box::new(Type::Mixed),
            };
        }

        let mut key_types = Vec::new();
        let mut value_types = Vec::new();
        let mut is_list = true;
        let mut expected_index = 0i64;

        for element in arr.elements.iter() {
            match element {
                ArrayElement::KeyValue(kv) => {
                    key_types.push(self.resolve(&kv.key, scope));
                    value_types.push(self.resolve(&kv.value, scope));
                    is_list = false;
                }
                ArrayElement::Value(val) => {
                    key_types.push(Type::ConstantInt(expected_index));
                    value_types.push(self.resolve(&val.value, scope));
                    expected_index += 1;
                }
                ArrayElement::Variadic(_) => {
                    is_list = false;
                }
                ArrayElement::Missing(_) => {
                    expected_index += 1;
                }
            }
        }

        let key_type = if key_types.is_empty() {
            Type::Mixed
        } else if key_types.iter().all(|t| matches!(t, Type::ConstantInt(_))) && is_list {
            Type::Int
        } else {
            // Generalize to common type
            Type::Mixed
        };

        let value_type = if value_types.is_empty() {
            Type::Mixed
        } else if value_types.len() == 1 {
            value_types.into_iter().next().unwrap().generalize()
        } else {
            // Union of all value types (simplified)
            value_types.into_iter().fold(Type::Never, |acc, t| acc.union_with(t)).generalize()
        };

        if is_list {
            Type::List {
                value: Box::new(value_type),
            }
        } else {
            Type::Array {
                key: Box::new(key_type),
                value: Box::new(value_type),
            }
        }
    }

    /// Resolve legacy array (array(...)) type
    fn resolve_legacy_array(&self, arr: &LegacyArrayExpression, scope: &Scope) -> Type {
        if arr.elements.is_empty() {
            return Type::mixed_array();
        }

        // Similar logic to resolve_array
        Type::mixed_array()
    }

    /// Resolve array access type
    fn resolve_array_access(&self, access: &ArrayAccessExpression, scope: &Scope) -> Type {
        let array_type = self.resolve(&access.array, scope);

        match array_type {
            Type::Array { value, .. } | Type::List { value } | Type::NonEmptyArray { value, .. } => {
                *value
            }
            Type::String | Type::ConstantString(_) | Type::NonEmptyString => Type::String,
            _ => Type::Mixed,
        }
    }

    /// Resolve closure type
    fn resolve_closure(&self, _closure: &ClosureExpression) -> Type {
        Type::Closure
    }

    /// Resolve new expression type
    fn resolve_new(&self, new: &NewExpression, scope: &Scope) -> Type {
        match &new.class {
            Expression::Name(name) => {
                let class_name = self.get_name_text(name);
                let resolved = scope.resolve_class_name(&class_name);
                Type::Object {
                    class_name: Some(resolved),
                }
            }
            Expression::Self_(_) => scope.get_this_type().unwrap_or(Type::SelfType),
            Expression::Static(_) => Type::Static,
            Expression::Parent(_) => Type::Parent,
            _ => Type::Object { class_name: None },
        }
    }

    /// Resolve object operation (method call, property access)
    fn resolve_object_operation(&self, op: &ObjectOperationExpression, scope: &Scope) -> Type {
        let object_type = self.resolve(&op.object, scope);

        match &op.kind {
            ObjectOperationKind::PropertyFetch(pf) => {
                self.resolve_property_fetch(&object_type, &pf.property)
            }
            ObjectOperationKind::NullsafePropertyFetch(pf) => {
                let inner = self.resolve_property_fetch(&object_type, &pf.property);
                Type::Nullable(Box::new(inner))
            }
            ObjectOperationKind::MethodCall(mc) => {
                self.resolve_method_call(&object_type, &mc.method)
            }
            ObjectOperationKind::NullsafeMethodCall(mc) => {
                let inner = self.resolve_method_call(&object_type, &mc.method);
                Type::Nullable(Box::new(inner))
            }
            ObjectOperationKind::MethodClosureCreation(_) => Type::Closure,
        }
    }

    /// Resolve property fetch type
    fn resolve_property_fetch(&self, object_type: &Type, property: &ClassLikeMemberSelector) -> Type {
        if let Type::Object { class_name: Some(class_name) } = object_type {
            if let ClassLikeMemberSelector::Identifier(ident) = property {
                let prop_name = self.get_span_text(&ident.span);
                if let Some(class) = self.symbol_table.get_class(class_name) {
                    if let Some(prop) = class.get_property(&prop_name) {
                        return prop.type_.clone().unwrap_or(Type::Mixed);
                    }
                }
            }
        }
        Type::Mixed
    }

    /// Resolve method call type
    fn resolve_method_call(&self, object_type: &Type, method: &ClassLikeMemberSelector) -> Type {
        if let Type::Object { class_name: Some(class_name) } = object_type {
            if let ClassLikeMemberSelector::Identifier(ident) = method {
                let method_name = self.get_span_text(&ident.span);
                if let Some(class) = self.symbol_table.get_class(class_name) {
                    if let Some(method_info) = class.get_method(&method_name) {
                        return method_info.return_type.clone().unwrap_or(Type::Mixed);
                    }
                }
            }
        }
        Type::Mixed
    }

    /// Resolve class operation (static method call, constant access)
    fn resolve_class_operation(&self, op: &ClassOperationExpression, scope: &Scope) -> Type {
        let class_name = match &op.class {
            Expression::Name(name) => self.get_name_text(name),
            Expression::Self_(_) => {
                scope.class_context().map_or("self".to_string(), |c| c.name.clone())
            }
            Expression::Static(_) => "static".to_string(),
            Expression::Parent(_) => "parent".to_string(),
            _ => return Type::Mixed,
        };

        match &op.kind {
            ClassOperationKind::ConstantFetch(cf) => {
                if let ClassLikeMemberSelector::Identifier(ident) = &cf.constant {
                    let const_name = self.get_span_text(&ident.span);
                    if const_name == "class" {
                        return Type::ClassString {
                            class_name: Some(class_name),
                        };
                    }
                    if let Some(class) = self.symbol_table.get_class(&class_name) {
                        if let Some(constant) = class.get_constant(&const_name) {
                            return constant.type_.clone().unwrap_or(Type::Mixed);
                        }
                    }
                }
                Type::Mixed
            }
            ClassOperationKind::StaticPropertyFetch(spf) => {
                if let ClassLikeMemberSelector::Variable(var) = &spf.property {
                    if let Variable::Direct(direct) = var.as_ref() {
                        let prop_name = self.get_span_text(&direct.name.span).trim_start_matches('$');
                        if let Some(class) = self.symbol_table.get_class(&class_name) {
                            if let Some(prop) = class.get_property(prop_name) {
                                return prop.type_.clone().unwrap_or(Type::Mixed);
                            }
                        }
                    }
                }
                Type::Mixed
            }
            ClassOperationKind::StaticMethodCall(smc) => {
                if let ClassLikeMemberSelector::Identifier(ident) = &smc.method {
                    let method_name = self.get_span_text(&ident.span);
                    if let Some(class) = self.symbol_table.get_class(&class_name) {
                        if let Some(method) = class.get_method(&method_name) {
                            return method.return_type.clone().unwrap_or(Type::Mixed);
                        }
                    }
                }
                Type::Mixed
            }
            ClassOperationKind::StaticMethodClosureCreation(_) => Type::Closure,
        }
    }

    /// Resolve function call type
    fn resolve_call(&self, call: &CallExpression, scope: &Scope) -> Type {
        if let Expression::Name(name) = &call.target {
            let func_name = self.get_name_text(name);

            // Check built-in function return types
            match func_name.to_lowercase().as_str() {
                "strlen" | "count" | "sizeof" => return Type::Int,
                "strval" | "trim" | "ltrim" | "rtrim" | "strtolower" | "strtoupper" => return Type::String,
                "intval" => return Type::Int,
                "floatval" | "doubleval" => return Type::Float,
                "boolval" => return Type::Bool,
                "array_keys" => return Type::list(Type::Mixed),
                "array_values" => return Type::list(Type::Mixed),
                "array_merge" | "array_replace" => return Type::mixed_array(),
                "array_filter" | "array_map" | "array_reverse" => return Type::mixed_array(),
                "array_pop" | "array_shift" => return Type::Mixed,
                "is_null" | "is_array" | "is_string" | "is_int" | "is_float" | "is_bool"
                | "is_object" | "is_callable" | "is_numeric" | "isset" | "empty" => return Type::Bool,
                "json_encode" => return Type::union(vec![Type::String, Type::ConstantBool(false)]),
                "json_decode" => return Type::Mixed,
                "file_get_contents" => return Type::union(vec![Type::String, Type::ConstantBool(false)]),
                "file_exists" | "is_file" | "is_dir" | "is_readable" | "is_writable" => return Type::Bool,
                "class_exists" | "method_exists" | "property_exists" | "function_exists" => return Type::Bool,
                "get_class" => return Type::union(vec![Type::String, Type::ConstantBool(false)]),
                "gettype" => return Type::String,
                "time" | "strtotime" => return Type::Int,
                "microtime" => return Type::union(vec![Type::Float, Type::String]),
                "date" | "gmdate" => return Type::String,
                "sprintf" | "vsprintf" => return Type::String,
                "preg_match" | "preg_match_all" => return Type::union(vec![Type::Int, Type::ConstantBool(false)]),
                "preg_replace" => return Type::union(vec![Type::String, Type::mixed_array(), Type::Null]),
                _ => {}
            }

            // Check symbol table
            if let Some(func) = self.symbol_table.get_function(&func_name) {
                return func.return_type.clone().unwrap_or(Type::Mixed);
            }
        }

        Type::Mixed
    }

    /// Resolve ternary expression type
    fn resolve_ternary(&self, ternary: &TernaryExpression, scope: &Scope) -> Type {
        match ternary {
            TernaryExpression::Ternary(t) => {
                let if_true = t.if_true.as_ref()
                    .map(|e| self.resolve(e, scope))
                    .unwrap_or_else(|| self.resolve(&t.condition, scope));
                let if_false = self.resolve(&t.if_false, scope);
                if_true.union_with(if_false)
            }
            TernaryExpression::ShortTernary(t) => {
                let condition_type = self.resolve(&t.condition, scope);
                let if_false = self.resolve(&t.if_false, scope);
                condition_type.union_with(if_false)
            }
        }
    }

    /// Resolve null coalesce expression type
    fn resolve_coalesce(&self, coalesce: &CoalesceExpression, scope: &Scope) -> Type {
        let left = self.resolve(&coalesce.lhs, scope).remove_null();
        let right = self.resolve(&coalesce.rhs, scope);
        left.union_with(right)
    }

    /// Resolve match expression type
    fn resolve_match(&self, m: &MatchExpression, scope: &Scope) -> Type {
        let mut result_type = Type::Never;
        for arm in m.arms.iter() {
            let arm_type = self.resolve(&arm.expression, scope);
            result_type = result_type.union_with(arm_type);
        }
        result_type
    }

    /// Resolve binary expression type
    fn resolve_binary(&self, binary: &BinaryExpression, scope: &Scope) -> Type {
        match &binary.operator {
            BinaryOperator::Addition(_) | BinaryOperator::Subtraction(_)
            | BinaryOperator::Multiplication(_) | BinaryOperator::Division(_)
            | BinaryOperator::Modulo(_) | BinaryOperator::Exponentiation(_) => {
                let left = self.resolve(&binary.lhs, scope);
                let right = self.resolve(&binary.rhs, scope);
                // Simplified: if both are int, result is int/float; otherwise mixed
                if matches!(left, Type::Int | Type::ConstantInt(_))
                    && matches!(right, Type::Int | Type::ConstantInt(_)) {
                    if matches!(binary.operator, BinaryOperator::Division(_)) {
                        Type::union(vec![Type::Int, Type::Float])
                    } else {
                        Type::Int
                    }
                } else {
                    Type::union(vec![Type::Int, Type::Float])
                }
            }
            BinaryOperator::Concatenation(_) => Type::String,
            BinaryOperator::BitwiseAnd(_) | BinaryOperator::BitwiseOr(_)
            | BinaryOperator::BitwiseXor(_) | BinaryOperator::LeftShift(_)
            | BinaryOperator::RightShift(_) => Type::Int,
            BinaryOperator::Equal(_) | BinaryOperator::Identical(_)
            | BinaryOperator::NotEqual(_) | BinaryOperator::NotIdentical(_)
            | BinaryOperator::LessThan(_) | BinaryOperator::LessThanOrEqual(_)
            | BinaryOperator::GreaterThan(_) | BinaryOperator::GreaterThanOrEqual(_)
            | BinaryOperator::Spaceship(_) => Type::Bool,
            BinaryOperator::And(_) | BinaryOperator::Or(_)
            | BinaryOperator::LowAnd(_) | BinaryOperator::LowOr(_)
            | BinaryOperator::LowXor(_) => Type::Bool,
            BinaryOperator::Instanceof(_) => Type::Bool,
        }
    }

    /// Resolve unary expression type
    fn resolve_unary(&self, unary: &UnaryExpression, scope: &Scope) -> Type {
        match &unary.operator {
            UnaryOperator::Not(_) | UnaryOperator::LogicalNot(_) => Type::Bool,
            UnaryOperator::BitwiseNot(_) => Type::Int,
            UnaryOperator::Negative(_) | UnaryOperator::Positive(_) => {
                let inner = self.resolve(&unary.operand, scope);
                match inner {
                    Type::Int | Type::ConstantInt(_) => Type::Int,
                    Type::Float | Type::ConstantFloat(_) => Type::Float,
                    _ => Type::union(vec![Type::Int, Type::Float]),
                }
            }
            UnaryOperator::PreIncrement(_) | UnaryOperator::PreDecrement(_)
            | UnaryOperator::PostIncrement(_) | UnaryOperator::PostDecrement(_) => {
                let inner = self.resolve(&unary.operand, scope);
                match inner {
                    Type::Int | Type::ConstantInt(_) => Type::Int,
                    Type::Float | Type::ConstantFloat(_) => Type::Float,
                    _ => Type::Mixed,
                }
            }
            UnaryOperator::ErrorControl(_) => self.resolve(&unary.operand, scope),
        }
    }

    /// Resolve magic constant type
    fn resolve_magic_constant(&self, mc: &MagicConstant) -> Type {
        match mc {
            MagicConstant::Line(_) => Type::Int,
            MagicConstant::File(_) | MagicConstant::Directory(_)
            | MagicConstant::Function(_) | MagicConstant::Class(_)
            | MagicConstant::Method(_) | MagicConstant::Namespace(_)
            | MagicConstant::Trait(_) | MagicConstant::Property(_) => Type::String,
        }
    }

    /// Resolve cast expression type
    fn resolve_cast(&self, cast: &CastExpression) -> Type {
        match &cast.kind {
            CastKind::Int(_) => Type::Int,
            CastKind::Bool(_) => Type::Bool,
            CastKind::Float(_) | CastKind::Double(_) => Type::Float,
            CastKind::String(_) => Type::String,
            CastKind::Binary(_) => Type::String,
            CastKind::Array(_) => Type::mixed_array(),
            CastKind::Object(_) => Type::Object { class_name: None },
            CastKind::Unset(_) => Type::Null,
        }
    }

    /// Parse a type hint node into a Type
    pub fn resolve_type_hint(&self, hint: &Hint, scope: &Scope) -> Type {
        match hint {
            Hint::Void(_) => Type::Void,
            Hint::Never(_) => Type::Never,
            Hint::Null(_) => Type::Null,
            Hint::True(_) => Type::ConstantBool(true),
            Hint::False(_) => Type::ConstantBool(false),
            Hint::Bool(_) => Type::Bool,
            Hint::Integer(_) => Type::Int,
            Hint::Float(_) => Type::Float,
            Hint::String(_) => Type::String,
            Hint::Array(_) => Type::mixed_array(),
            Hint::Object(_) => Type::Object { class_name: None },
            Hint::Mixed(_) => Type::Mixed,
            Hint::Callable(_) => Type::Callable,
            Hint::Iterable(_) => Type::Iterable {
                key: Box::new(Type::Mixed),
                value: Box::new(Type::Mixed),
            },
            Hint::Static(_) => Type::Static,
            Hint::Self_(_) => Type::SelfType,
            Hint::Parent(_) => Type::Parent,
            Hint::Identifier(ident) => {
                let name = self.get_span_text(&ident.span);
                let resolved = scope.resolve_class_name(&name);
                Type::Object {
                    class_name: Some(resolved),
                }
            }
            Hint::Nullable(nullable) => {
                let inner = self.resolve_type_hint(&nullable.hint, scope);
                Type::Nullable(Box::new(inner))
            }
            Hint::Union(union) => {
                let left = self.resolve_type_hint(&union.left, scope);
                let right = self.resolve_type_hint(&union.right, scope);
                left.union_with(right)
            }
            Hint::Intersection(intersection) => {
                let left = self.resolve_type_hint(&intersection.left, scope);
                let right = self.resolve_type_hint(&intersection.right, scope);
                left.intersect_with(right)
            }
            Hint::Parenthesized(p) => self.resolve_type_hint(&p.hint, scope),
        }
    }

    /// Get text for a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Get text for a Name
    fn get_name_text(&self, name: &Name) -> String {
        match name {
            Name::Resolved(resolved) => {
                self.get_span_text(&resolved.span).to_string()
            }
            Name::Unresolved(unresolved) => {
                let parts: Vec<_> = unresolved.parts.iter()
                    .map(|p| self.get_span_text(&p.span))
                    .collect();
                parts.join("\\")
            }
        }
    }
}
