//! rscript_lsp: Language Server Protocol implementation.
//!
//! Implements the LSP protocol for editor integration, powered by
//! the language service.

#![allow(clippy::needless_update)]

use rscript_ls::LanguageService;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// The LSP backend.
pub struct RscriptLspServer {
    client: Client,
    language_service: Mutex<LanguageService>,
}

impl RscriptLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            language_service: Mutex::new(LanguageService::new()),
        }
    }

    fn uri_to_key(uri: &Url) -> String {
        uri.to_string()
    }

    fn position_to_offset(text: &str, position: Position) -> u32 {
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, ch) in text.char_indices() {
            if line == position.line && col == position.character {
                return i as u32;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        text.len() as u32
    }

    fn offset_to_position(text: &str, offset: u32) -> Position {
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, ch) in text.char_indices() {
            if i as u32 == offset {
                return Position::new(line, col);
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position::new(line, col)
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let diagnostics = {
            let mut ls = self.language_service.lock().unwrap();
            let key = Self::uri_to_key(&uri);
            let diags = ls.get_diagnostics(&key);
            let text = ls.get_document_text(&key).unwrap_or("").to_string();
            diags.into_iter().map(|d| {
                let start_pos = if let Some(span) = d.span {
                    Self::offset_to_position(&text, span.start)
                } else {
                    Position::new(0, 0)
                };
                let end_pos = if let Some(span) = d.span {
                    Self::offset_to_position(&text, span.start + span.length)
                } else {
                    Position::new(0, 0)
                };
                Diagnostic {
                    range: Range::new(start_pos, end_pos),
                    severity: Some(if d.is_error() {
                        DiagnosticSeverity::ERROR
                    } else {
                        DiagnosticSeverity::WARNING
                    }),
                    code: Some(NumberOrString::Number(d.code as i32)),
                    source: Some("rsc".to_string()),
                    message: d.message_text,
                    ..Default::default()
                }
            }).collect::<Vec<_>>()
        };

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for RscriptLspServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    }
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "\"".to_string(),
                        "'".to_string(),
                        "/".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "rscript".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "rscript language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let key = Self::uri_to_key(&uri);
        {
            let mut ls = self.language_service.lock().unwrap();
            ls.open_document(
                key,
                params.text_document.text,
                params.text_document.version,
            );
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let key = Self::uri_to_key(&uri);
        {
            let mut ls = self.language_service.lock().unwrap();
            // We use FULL sync, so take the last change
            if let Some(change) = params.content_changes.into_iter().last() {
                ls.update_document(&key, change.text, params.text_document.version);
            }
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let key = Self::uri_to_key(&params.text_document.uri);
        let mut ls = self.language_service.lock().unwrap();
        ls.close_document(&key);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.publish_diagnostics(params.text_document.uri).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let key = Self::uri_to_key(&uri);

        let items = {
            let ls = self.language_service.lock().unwrap();
            let text = ls.get_document_text(&key).unwrap_or("").to_string();
            let offset = Self::position_to_offset(&text, position);
            ls.get_completions(&key, offset)
        };

        let lsp_items: Vec<CompletionItem> = items.into_iter().map(|item| {
            CompletionItem {
                label: item.label,
                kind: Some(to_lsp_completion_kind(item.kind)),
                detail: item.detail,
                insert_text: item.insert_text,
                sort_text: item.sort_text,
                ..Default::default()
            }
        }).collect();

        Ok(Some(CompletionResponse::Array(lsp_items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let key = Self::uri_to_key(&uri);

        let hover_info = {
            let ls = self.language_service.lock().unwrap();
            let text = ls.get_document_text(&key).unwrap_or("").to_string();
            let offset = Self::position_to_offset(&text, position);
            ls.get_hover(&key, offset)
        };

        Ok(hover_info.map(|info| {
            Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```typescript\n{}\n```", info.contents),
                }),
                range: None,
            }
        }))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let key = Self::uri_to_key(&uri);

        let definitions = {
            let ls = self.language_service.lock().unwrap();
            let text = ls.get_document_text(&key).unwrap_or("").to_string();
            let offset = Self::position_to_offset(&text, position);
            ls.get_definition(&key, offset)
        };

        if definitions.is_empty() {
            return Ok(None);
        }

        let locations: Vec<Location> = definitions.into_iter().filter_map(|def| {
            let def_uri = Url::parse(&def.file_name).ok()?;
            Some(Location {
                uri: def_uri,
                range: Range::new(
                    Position::new(0, def.span.start),
                    Position::new(0, def.span.start + def.span.length),
                ),
            })
        }).collect();

        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let key = Self::uri_to_key(&uri);

        let refs = {
            let ls = self.language_service.lock().unwrap();
            let text = ls.get_document_text(&key).unwrap_or("").to_string();
            let offset = Self::position_to_offset(&text, position);
            let refs = ls.get_references(&key, offset);
            refs.into_iter().map(|r| {
                let start = Self::offset_to_position(&text, r.span.start);
                let end = Self::offset_to_position(&text, r.span.start + r.span.length);
                (r.file_name, start, end)
            }).collect::<Vec<_>>()
        };

        let locations: Vec<Location> = refs.into_iter().map(|(file, start, end)| {
            let ref_uri = Url::parse(&file).ok().unwrap_or_else(|| uri.clone());
            Location {
                uri: ref_uri,
                range: Range::new(start, end),
            }
        }).collect();

        Ok(Some(locations))
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let key = Self::uri_to_key(&uri);

        let symbols = {
            let ls = self.language_service.lock().unwrap();
            ls.get_document_symbols(&key)
        };

        let lsp_symbols: Vec<SymbolInformation> = symbols.into_iter().enumerate().map(|(i, sym)| {
            #[allow(deprecated)]
            SymbolInformation {
                name: sym.name,
                kind: to_lsp_symbol_kind(sym.kind),
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: Range::new(
                        Position::new(i as u32, 0),
                        Position::new(i as u32, 0),
                    ),
                },
                container_name: None,
            }
        }).collect();

        Ok(Some(DocumentSymbolResponse::Flat(lsp_symbols)))
    }
}

fn to_lsp_completion_kind(kind: rscript_ls::CompletionItemKind) -> CompletionItemKind {
    match kind {
        rscript_ls::CompletionItemKind::Variable => CompletionItemKind::VARIABLE,
        rscript_ls::CompletionItemKind::Function => CompletionItemKind::FUNCTION,
        rscript_ls::CompletionItemKind::Class => CompletionItemKind::CLASS,
        rscript_ls::CompletionItemKind::Interface => CompletionItemKind::INTERFACE,
        rscript_ls::CompletionItemKind::Module => CompletionItemKind::MODULE,
        rscript_ls::CompletionItemKind::Property => CompletionItemKind::PROPERTY,
        rscript_ls::CompletionItemKind::Method => CompletionItemKind::METHOD,
        rscript_ls::CompletionItemKind::Keyword => CompletionItemKind::KEYWORD,
        rscript_ls::CompletionItemKind::Type => CompletionItemKind::STRUCT,
        rscript_ls::CompletionItemKind::Enum => CompletionItemKind::ENUM,
        rscript_ls::CompletionItemKind::EnumMember => CompletionItemKind::ENUM_MEMBER,
        rscript_ls::CompletionItemKind::Constant => CompletionItemKind::CONSTANT,
    }
}

fn to_lsp_symbol_kind(kind: rscript_ls::DocumentSymbolKind) -> SymbolKind {
    match kind {
        rscript_ls::DocumentSymbolKind::File => SymbolKind::FILE,
        rscript_ls::DocumentSymbolKind::Module => SymbolKind::MODULE,
        rscript_ls::DocumentSymbolKind::Namespace => SymbolKind::NAMESPACE,
        rscript_ls::DocumentSymbolKind::Class => SymbolKind::CLASS,
        rscript_ls::DocumentSymbolKind::Method => SymbolKind::METHOD,
        rscript_ls::DocumentSymbolKind::Property => SymbolKind::PROPERTY,
        rscript_ls::DocumentSymbolKind::Function => SymbolKind::FUNCTION,
        rscript_ls::DocumentSymbolKind::Variable => SymbolKind::VARIABLE,
        rscript_ls::DocumentSymbolKind::Constant => SymbolKind::CONSTANT,
        rscript_ls::DocumentSymbolKind::Enum => SymbolKind::ENUM,
        rscript_ls::DocumentSymbolKind::Interface => SymbolKind::INTERFACE,
        rscript_ls::DocumentSymbolKind::TypeParameter => SymbolKind::TYPE_PARAMETER,
        rscript_ls::DocumentSymbolKind::EnumMember => SymbolKind::ENUM_MEMBER,
    }
}

/// Start the LSP server.
pub async fn start_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(RscriptLspServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
