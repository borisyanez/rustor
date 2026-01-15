//! Symbol collector for building symbol table from AST
//!
//! Collects class, function, and constant definitions from PHP files.
//! This is a simplified version that extracts basic symbol information.

use crate::symbols::{ClassInfo, FunctionInfo, SymbolTable};
use crate::symbols::class_info::ClassKind;
use crate::types::Type;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Collects symbols from PHP AST using the Visitor pattern
pub struct SymbolCollector<'s> {
    source: &'s str,
    file: PathBuf,
    current_namespace: Option<String>,
    symbols: CollectedSymbols,
    /// Current use aliases being collected
    current_aliases: HashMap<String, String>,
}

impl<'s> SymbolCollector<'s> {
    /// Create a new symbol collector
    pub fn new(source: &'s str, file: &Path) -> Self {
        let mut symbols = CollectedSymbols::default();
        symbols.file_path = file.to_path_buf();

        Self {
            source,
            file: file.to_path_buf(),
            current_namespace: None,
            symbols,
            current_aliases: HashMap::new(),
        }
    }

    /// Collect all symbols from a program
    pub fn collect(mut self, program: &Program<'_>) -> CollectedSymbols {
        self.visit_program(program, self.source);
        // Store collected aliases
        self.symbols.aliases = self.current_aliases;
        self.symbols
    }

    /// Build a symbol table from collected symbols
    pub fn build_symbol_table_from_symbols(collected: Vec<CollectedSymbols>) -> SymbolTable {
        let mut table = SymbolTable::with_builtins();

        for symbols in collected {
            // Register aliases for this file
            if !symbols.aliases.is_empty() {
                table.set_aliases(&symbols.file_path, symbols.aliases);
            }

            for class in symbols.classes {
                table.register_class(class);
            }
            for func in symbols.functions {
                table.register_function(func);
            }
            for (name, ty) in symbols.constants {
                table.register_constant(name, ty);
            }
        }

        table
    }

    /// Get text for a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Qualify a name with the current namespace
    fn qualify_name(&self, name: &str) -> String {
        if name.starts_with('\\') {
            name[1..].to_string()
        } else if let Some(ns) = &self.current_namespace {
            format!("{}\\{}", ns, name)
        } else {
            name.to_string()
        }
    }

    /// Get line number from offset
    fn get_line(&self, offset: usize) -> usize {
        let mut line = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
            }
        }
        line
    }

    /// Extract type imports from use statement text
    fn extract_imports_from_use_text(&mut self, use_text: &str) {
        // Remove 'use', 'function', 'const' keywords and semicolon
        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        // Handle grouped imports: Foo\{Bar, Baz as Qux}
        if let Some(brace_start) = text.find('{') {
            if let Some(brace_end) = text.find('}') {
                let prefix = text[..brace_start].trim().trim_end_matches('\\');
                let group_content = &text[brace_start + 1..brace_end];

                for item in group_content.split(',') {
                    let item = item.trim();
                    if item.is_empty() {
                        continue;
                    }

                    // Handle "Bar as Baz" - use alias
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let name = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let full_name = if prefix.is_empty() {
                            name.to_string()
                        } else {
                            format!("{}\\{}", prefix, name)
                        };
                        self.current_aliases.insert(alias.to_string(), full_name);
                    } else {
                        // No alias, use last part of name
                        let full_name = if prefix.is_empty() {
                            item.to_string()
                        } else {
                            format!("{}\\{}", prefix, item)
                        };
                        let alias = item.rsplit('\\').next().unwrap_or(item);
                        self.current_aliases.insert(alias.to_string(), full_name);
                    }
                }
                return;
            }
        }

        // Handle single import: Foo\Bar or Foo\Bar as Baz
        if let Some(as_pos) = text.to_lowercase().find(" as ") {
            let full_name = text[..as_pos].trim().to_string();
            let alias = text[as_pos + 4..].trim().to_string();
            self.current_aliases.insert(alias, full_name);
        } else {
            // No alias, use last part of name
            let full_name = text.to_string();
            let alias = text.rsplit('\\').next().unwrap_or(text);
            self.current_aliases.insert(alias.to_string(), full_name);
        }
    }

    /* TODO: Re-implement with correct mago-syntax types
    /// Process use statement to extract type aliases
    fn process_use_statement(&mut self, use_stmt: &UseStatement<'_>) {
        match use_stmt {
            UseStatement::Default(default) => {
                for item in default.items.iter() {
                    match item {
                        UseItem::TypeAlias(alias) => {
                            // Get the full name being imported
                            let full_name = self.get_span_text(&alias.name.span()).to_string();

                            // Get the alias (if specified) or use the last part of the name
                            let alias_name = alias.alias.as_ref()
                                .map(|a| self.get_span_text(&a.alias.span).to_string())
                                .unwrap_or_else(|| {
                                    full_name.rsplit('\\').next().unwrap_or(&full_name).to_string()
                                });

                            // Store: alias -> fully qualified name
                            self.current_aliases.insert(alias_name, full_name);
                        }
                        UseItem::TypeGroup(group) => {
                            // Get the prefix namespace
                            let prefix = self.get_span_text(&group.namespace.span()).to_string();

                            for item in group.items.iter() {
                                match item {
                                    UseGroupItem::Alias(alias) => {
                                        let name = self.get_span_text(&alias.name.span);
                                        let full_name = format!("{}\\{}", prefix, name);

                                        let alias_name = alias.alias.as_ref()
                                            .map(|a| self.get_span_text(&a.alias.span).to_string())
                                            .unwrap_or_else(|| name.to_string());

                                        self.current_aliases.insert(alias_name, full_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    */
}

impl<'a, 's> Visitor<'a> for SymbolCollector<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        match stmt {
            Statement::Namespace(ns) => {
                // Extract namespace from the namespace statement span
                let ns_span = ns.span();
                let ns_text = self.get_span_text(&ns_span);
                // Try to extract namespace name from the text
                if let Some(name_start) = ns_text.find("namespace") {
                    let after_keyword = &ns_text[name_start + 9..];
                    let name_end = after_keyword.find(|c: char| c == '{' || c == ';')
                        .unwrap_or(after_keyword.len());
                    let name = after_keyword[..name_end].trim();
                    if !name.is_empty() {
                        self.current_namespace = Some(name.to_string());
                    }
                }
                true
            }
            Statement::Use(use_stmt) => {
                // Extract use statement text and parse it simply
                let use_span = use_stmt.span();
                let use_text = self.get_span_text(&use_span).to_string();
                self.extract_imports_from_use_text(&use_text);
                true
            }
            Statement::Function(func) => {
                let span = func.name.span;
                let name = self.get_span_text(&span);
                let full_name = self.qualify_name(name);

                let mut info = FunctionInfo::new(name, &full_name);
                info.file = Some(self.file.clone());
                info.line = Some(self.get_line(span.start.offset as usize));

                self.symbols.functions.push(info);
                true
            }
            Statement::Class(class) => {
                let span = class.name.span;
                let name = self.get_span_text(&span);
                let full_name = self.qualify_name(name);

                let mut info = ClassInfo::new(name, &full_name);
                info.kind = ClassKind::Class;
                info.file = Some(self.file.clone());
                info.line = Some(self.get_line(span.start.offset as usize));

                // Extract extends (parent class) - classes extend only one parent
                if let Some(extends) = &class.extends {
                    if let Some(parent) = extends.types.first() {
                        let parent_text = self.get_span_text(&parent.span());
                        info.parent = Some(self.qualify_name(parent_text));
                    }
                }

                // Extract implements (interfaces)
                if let Some(implements) = &class.implements {
                    for iface in implements.types.iter() {
                        let iface_text = self.get_span_text(&iface.span());
                        info.interfaces.push(self.qualify_name(iface_text));
                    }
                }

                self.symbols.classes.push(info);
                true
            }
            Statement::Interface(interface) => {
                let span = interface.name.span;
                let name = self.get_span_text(&span);
                let full_name = self.qualify_name(name);

                let mut info = ClassInfo::new(name, &full_name);
                info.kind = ClassKind::Interface;
                info.file = Some(self.file.clone());
                info.line = Some(self.get_line(span.start.offset as usize));

                // Extract extends (interfaces can extend other interfaces)
                if let Some(extends) = &interface.extends {
                    for parent_iface in extends.types.iter() {
                        let parent_text = self.get_span_text(&parent_iface.span());
                        info.interfaces.push(self.qualify_name(parent_text));
                    }
                }

                self.symbols.classes.push(info);
                true
            }
            Statement::Trait(trait_def) => {
                let span = trait_def.name.span;
                let name = self.get_span_text(&span);
                let full_name = self.qualify_name(name);

                let mut info = ClassInfo::new(name, &full_name);
                info.kind = ClassKind::Trait;
                info.file = Some(self.file.clone());
                info.line = Some(self.get_line(span.start.offset as usize));

                self.symbols.classes.push(info);
                true
            }
            Statement::Enum(enum_def) => {
                let span = enum_def.name.span;
                let name = self.get_span_text(&span);
                let full_name = self.qualify_name(name);

                let mut info = ClassInfo::new(name, &full_name);
                info.kind = ClassKind::Enum;
                info.file = Some(self.file.clone());
                info.line = Some(self.get_line(span.start.offset as usize));

                self.symbols.classes.push(info);
                true
            }
            Statement::Constant(const_def) => {
                for entry in const_def.items.iter() {
                    let name = self.get_span_text(&entry.name.span);
                    let full_name = self.qualify_name(name);
                    self.symbols.constants.push((full_name, Type::Mixed));
                }
                true
            }
            _ => true,
        }
    }
}

/// Symbols collected from a file
#[derive(Debug, Default)]
pub struct CollectedSymbols {
    pub classes: Vec<ClassInfo>,
    pub functions: Vec<FunctionInfo>,
    pub constants: Vec<(String, Type)>,
    /// File path where these symbols were collected from
    pub file_path: PathBuf,
    /// Use statement aliases (short name -> fully qualified name)
    pub aliases: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mago_database::file::FileId;

    fn parse_and_collect(source: &str) -> CollectedSymbols {
        let arena = Box::leak(Box::new(bumpalo::Bump::new()));
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(arena, file_id, source);
        let collector = SymbolCollector::new(source, Path::new("test.php"));
        collector.collect(&program)
    }

    #[test]
    fn test_collect_function() {
        let source = r#"<?php
function my_function() {
}
"#;
        let symbols = parse_and_collect(source);
        assert_eq!(symbols.functions.len(), 1);
        assert_eq!(symbols.functions[0].name, "my_function");
    }

    #[test]
    fn test_collect_class() {
        let source = r#"<?php
class User {
}
"#;
        let symbols = parse_and_collect(source);
        assert_eq!(symbols.classes.len(), 1);
        assert_eq!(symbols.classes[0].name, "User");
    }

    #[test]
    fn test_collect_interface() {
        let source = r#"<?php
interface Nameable {
}
"#;
        let symbols = parse_and_collect(source);
        assert_eq!(symbols.classes.len(), 1);
        assert_eq!(symbols.classes[0].name, "Nameable");
        assert_eq!(symbols.classes[0].kind, ClassKind::Interface);
    }

    #[test]
    fn test_collect_with_namespace() {
        let source = r#"<?php
namespace App\Models;

class User {
}
"#;
        let symbols = parse_and_collect(source);
        assert_eq!(symbols.classes.len(), 1);
        assert_eq!(symbols.classes[0].full_name, "App\\Models\\User");
    }
}
