//! LSP server for rustor IDE integration
//!
//! Provides real-time diagnostics and code actions for PHP files.
//!
//! Usage:
//!   rustor --lsp
//!
//! Configure in VS Code settings.json:
//! ```json
//! {
//!   "rustor.enable": true,
//!   "rustor.path": "/path/to/rustor"
//! }
//! ```

use std::collections::HashSet;

use bumpalo::Bump;
use mago_database::file::FileId;
use mago_span::HasSpan;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use rustor_rules::{RuleRegistry, Preset};

/// Rustor LSP server backend
pub struct RustorLsp {
    client: Client,
}

impl RustorLsp {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn check_document(&self, uri: &Url, text: &str) {
        // Do all synchronous work first (parsing and rule checking)
        let diagnostics = analyze_php_sync(text);

        // Then publish asynchronously
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

/// Synchronously analyze PHP source and return diagnostics
fn analyze_php_sync(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Parse the PHP file
    let arena = Bump::new();
    let file_id = FileId::new("buffer");
    let (program, parse_errors) = mago_syntax::parser::parse_file_content(&arena, file_id, source);

    // Report parse errors
    if let Some(error) = parse_errors {
        let span = error.span();
        let (start_line, start_col) = offset_to_line_col(source, span.start.offset as usize);
        let (end_line, end_col) = offset_to_line_col(source, span.end.offset as usize);

        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line: start_line as u32,
                    character: start_col as u32,
                },
                end: Position {
                    line: end_line as u32,
                    character: end_col as u32,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("rustor".to_string()),
            message: error.to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Run rustor rules
    let registry = RuleRegistry::new();
    let enabled: HashSet<String> = registry.get_preset_rules(Preset::Recommended);
    let edits = registry.check_all(program, source, &enabled);

    for edit in edits {
        let (start_line, start_col) = offset_to_line_col(source, edit.span.start.offset as usize);
        let (end_line, end_col) = offset_to_line_col(source, edit.span.end.offset as usize);

        // Extract rule name from message
        let rule_name = extract_rule_name(&edit.message);

        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line: start_line as u32,
                    character: start_col as u32,
                },
                end: Position {
                    line: end_line as u32,
                    character: end_col as u32,
                },
            },
            severity: Some(DiagnosticSeverity::HINT),
            code: Some(NumberOrString::String(rule_name.to_string())),
            code_description: None,
            source: Some("rustor".to_string()),
            message: edit.message.clone(),
            related_information: None,
            tags: None,
            data: Some(serde_json::json!({
                "replacement": edit.replacement,
                "start_offset": edit.span.start.offset,
                "end_offset": edit.span.end.offset,
            })),
        });
    }

    diagnostics
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn extract_rule_name(message: &str) -> &str {
    // Try to extract rule name from common message patterns
    if message.starts_with("Convert ") || message.starts_with("Replace ") {
        // Extract first word after common prefixes
        if let Some(paren_pos) = message.find(" (") {
            if let Some(last_word_start) = message[..paren_pos].rfind(' ') {
                return &message[last_word_start + 1..paren_pos];
            }
        }
    }
    "rustor"
}

#[tower_lsp::async_trait]
impl LanguageServer for RustorLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "rustor".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Rustor LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        if uri.path().ends_with(".php") {
            self.check_document(&uri, &text).await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if uri.path().ends_with(".php") {
            if let Some(change) = params.content_changes.into_iter().next() {
                self.check_document(&uri, &change.text).await;
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        if uri.path().ends_with(".php") {
            if let Some(text) = params.text {
                self.check_document(&uri, &text).await;
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear diagnostics when file is closed
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;

        if !uri.path().ends_with(".php") {
            return Ok(None);
        }

        let mut actions = Vec::new();

        // Convert diagnostics with rustor data into quick fixes
        for diagnostic in params.context.diagnostics {
            if diagnostic.source.as_deref() != Some("rustor") {
                continue;
            }

            if let Some(ref data) = diagnostic.data {
                if let Some(replacement) = data.get("replacement").and_then(|v| v.as_str()) {
                    let edit = TextEdit {
                        range: diagnostic.range,
                        new_text: replacement.to_string(),
                    };

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![edit]);

                    let action = CodeAction {
                        title: format!("Fix: {}", diagnostic.message),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }),
                        command: None,
                        is_preferred: Some(true),
                        disabled: None,
                        data: None,
                    };

                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

/// Run the LSP server
pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(RustorLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
