//! Check for access to undefined static properties (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for static property access like Foo::$bar
pub struct StaticPropertyCheck;

impl Check for StaticPropertyCheck {
    fn id(&self) -> &'static str {
        "staticProperty.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects access to undefined static properties"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = StaticPropertyVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_properties: HashMap::new(),
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            use_fqn_map: HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect class definitions with properties and use statements
        visitor.collect_definitions(program);

        // Second pass: check static property access
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct StaticPropertyVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_properties: HashMap<String, HashSet<String>>, // class name -> property names (without $)
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    use_fqn_map: HashMap<String, String>, // short name -> FQN
    issues: Vec<Issue>,
}

impl<'s> StaticPropertyVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt, "");
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>, namespace: &str) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span);
                let fqn = if namespace.is_empty() {
                    class_name.to_string()
                } else {
                    format!("{}\\{}", namespace, class_name)
                };
                let mut properties = HashSet::new();

                // Collect static properties
                for member in class.members.iter() {
                    if let ClassLikeMember::Property(Property::Plain(prop)) = member {
                        // Check if property is static
                        let is_static = prop.modifiers.iter().any(|m| {
                            matches!(m, Modifier::Static(_))
                        });
                        if is_static {
                            // Get property names from items
                            for item in prop.items.nodes.iter() {
                                let var_name = match item {
                                    PropertyItem::Abstract(abs) => &abs.variable.name,
                                    PropertyItem::Concrete(conc) => &conc.variable.name,
                                };
                                // Remove $ prefix
                                let prop_name = var_name.trim_start_matches('$').to_lowercase();
                                properties.insert(prop_name);
                            }
                        }
                    }
                }

                // Store both short name and FQN
                self.class_properties.insert(class_name.to_lowercase(), properties.clone());
                self.class_properties.insert(fqn.to_lowercase(), properties);
            }
            Statement::Trait(tr) => {
                let trait_name = self.get_span_text(&tr.name.span);
                let fqn = if namespace.is_empty() {
                    trait_name.to_string()
                } else {
                    format!("{}\\{}", namespace, trait_name)
                };
                let mut properties = HashSet::new();

                for member in tr.members.iter() {
                    if let ClassLikeMember::Property(Property::Plain(prop)) = member {
                        let is_static = prop.modifiers.iter().any(|m| {
                            matches!(m, Modifier::Static(_))
                        });
                        if is_static {
                            for item in prop.items.nodes.iter() {
                                let var_name = match item {
                                    PropertyItem::Abstract(abs) => &abs.variable.name,
                                    PropertyItem::Concrete(conc) => &conc.variable.name,
                                };
                                let prop_name = var_name.trim_start_matches('$').to_lowercase();
                                properties.insert(prop_name);
                            }
                        }
                    }
                }

                self.class_properties.insert(trait_name.to_lowercase(), properties.clone());
                self.class_properties.insert(fqn.to_lowercase(), properties);
            }
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Namespace(ns) => {
                let ns_name = if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.source[span.start.offset as usize..span.end.offset as usize].to_string()
                } else {
                    String::new()
                };

                if self.current_namespace.is_empty() {
                    self.current_namespace = ns_name.clone();
                }

                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.collect_from_stmt(inner, &ns_name);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_from_stmt(inner, &ns_name);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner, namespace);
                }
            }
            _ => {}
        }
    }

    fn collect_use_imports<'a>(&mut self, use_stmt: &Use<'a>) {
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];

        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let class_part = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let fqn = format!("{}\\{}", prefix, class_part);
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            let fqn = format!("{}\\{}", prefix, item.trim());
                            self.use_fqn_map.insert(name.to_lowercase(), fqn);
                        }
                    }
                }
            }
        } else if let Some(as_pos) = text.to_lowercase().find(" as ") {
            let fqn = text[..as_pos].trim().to_string();
            let alias = text[as_pos + 4..].trim();
            self.use_fqn_map.insert(alias.to_lowercase(), fqn);
        } else {
            let name = text.rsplit('\\').next().unwrap_or(text).trim();
            if !name.is_empty() {
                self.use_fqn_map.insert(name.to_lowercase(), text.to_string());
            }
        }
    }

    fn resolve_class_name(&self, name: &str) -> String {
        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        let name_lower = name.to_lowercase();

        if let Some(fqn) = self.use_fqn_map.get(&name_lower) {
            return fqn.clone();
        }

        if !self.current_namespace.is_empty() {
            format!("{}\\{}", self.current_namespace, name)
        } else {
            name.to_string()
        }
    }

    /// Check if a class has a static property
    /// Returns (found_class, has_property)
    fn class_has_property(&self, class_name: &str, property_name: &str) -> (bool, bool) {
        let class_lower = class_name.to_lowercase();
        let prop_lower = property_name.to_lowercase();

        // Check local definitions first
        if let Some(properties) = self.class_properties.get(&class_lower) {
            return (true, properties.contains(&prop_lower));
        }

        // Check symbol table
        if let Some(st) = self.symbol_table {
            if let Some(class_info) = st.get_class(&class_lower) {
                // Check direct property
                if class_info.has_property(&prop_lower) {
                    return (true, true);
                }

                // Check parent class hierarchy
                let mut current_parent = class_info.parent.clone();
                let mut depth = 0;
                while let Some(ref parent_name) = current_parent {
                    if depth > 20 {
                        break;
                    }
                    if let Some(parent_info) = st.get_class(&parent_name.to_lowercase()) {
                        if parent_info.has_property(&prop_lower) {
                            return (true, true);
                        }
                        current_parent = parent_info.parent.clone();
                    } else {
                        break;
                    }
                    depth += 1;
                }

                // Check trait properties
                let mut checked_traits = HashSet::new();
                let mut traits_to_check: Vec<String> = class_info.traits.clone();
                while let Some(trait_name) = traits_to_check.pop() {
                    let trait_lower = trait_name.to_lowercase();
                    if checked_traits.contains(&trait_lower) {
                        continue;
                    }
                    checked_traits.insert(trait_lower.clone());

                    if let Some(trait_info) = st.get_class(&trait_lower) {
                        if trait_info.has_property(&prop_lower) {
                            return (true, true);
                        }
                        for sub_trait in &trait_info.traits {
                            traits_to_check.push(sub_trait.clone());
                        }
                    }
                }

                // Class found but property not found
                return (true, false);
            }
        }

        // Class not found - can't verify
        (false, true)
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
}

impl<'a, 's> Visitor<'a> for StaticPropertyVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for Class::$property access
        if let Expression::Access(Access::StaticProperty(access)) = expr {
            // Get class name
            let class_name = match &*access.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            };

            // Get property name - StaticPropertyAccess has property field directly
            let prop_span = access.property.span();
            let prop_name = self.get_span_text(&prop_span).trim_start_matches('$');
            let property_info = Some((prop_name.to_string(), prop_span));

            if let (Some(class), Some((property, prop_span))) = (class_name, property_info) {
                // Skip self, static, parent
                if class.eq_ignore_ascii_case("self")
                    || class.eq_ignore_ascii_case("static")
                    || class.eq_ignore_ascii_case("parent")
                {
                    return true;
                }

                // Normalize class name
                let normalized_class = class.trim_start_matches('\\');

                // Skip built-in classes
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(normalized_class)) {
                    return true;
                }

                // Resolve class name to FQN
                let resolved_class = self.resolve_class_name(&class);

                // Check if the class has this property
                let (class_found, has_property) = self.class_has_property(&resolved_class, &property);

                // Only report if class is known and property doesn't exist
                if class_found && !has_property {
                    let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "staticProperty.notFound",
                            format!(
                                "Access to an undefined static property {}::${}.",
                                resolved_class, property
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("staticProperty.notFound"),
                    );
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_property_check_level() {
        let check = StaticPropertyCheck;
        assert_eq!(check.level(), 0);
    }
}
