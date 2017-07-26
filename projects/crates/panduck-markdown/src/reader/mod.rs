mod lexer;
mod parser;
mod token_type;

pub use self::token_type::{MarkdownToken, MarkdownTokenType};
pub use crate::ast::MarkdownRoot;
use crate::reader::lexer::MarkdownLexer;
use panduck_types::helpers::{check_path, SourceText};
use panduck_types::{PanduckDiagnostics, PanduckError};
use std::io::{read_to_string, Read};
use std::path::Path;
use url::Url;

#[derive(Copy, Clone, Debug)]
pub struct MarkdownReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct MarkdownReader<'input, R> {
    reader: R,
    config: &'input MarkdownReadConfig,
}

impl MarkdownReadConfig {
    pub fn reader<R>(&self, reader: R) -> MarkdownReader<R> {
        MarkdownReader {
            reader,
            config: self,
        }
    }
    pub fn read_str(&self, text: &str, url: Option<Url>) -> PanduckDiagnostics<MarkdownRoot> {
        self.reader(text.as_bytes()).read(url)
    }

    pub fn read_path(&self, path: impl AsRef<Path>) -> PanduckDiagnostics<MarkdownRoot> {
        match check_path(path) {
            Ok((file, url)) => self.reader(file).read(Some(url)),
            Err(e) => PanduckDiagnostics {
                result: Err(e),
                diagnostics: vec![],
            },
        }
    }
}

impl<'input, R: Read> MarkdownReader<'input, R> {
    pub fn read(mut self, url: Option<Url>) -> PanduckDiagnostics<MarkdownRoot> {
        let mut errors = vec![];
        let text = match read_to_string(&mut self.reader) {
            Ok(text) => text,
            Err(e) => {
                return PanduckDiagnostics {
                    result: Err(PanduckError::from(e)),
                    diagnostics: errors,
                };
            }
        };
        let source = SourceText::new(text, url);
        let mut lexer = MarkdownLexer {
            state: panduck_types::lexer::LexerState::new(&source),
            config: &self.config,
        };
        let PanduckDiagnostics {
            result,
            mut diagnostics,
        } = lexer.tokenizer();
        errors.append(&mut diagnostics);
        let tokens = match result {
            Ok(tokens) => tokens,
            Err(e) => {
                return PanduckDiagnostics {
                    result: Err(e),
                    diagnostics: errors,
                };
            }
        };
        let mut parser = crate::reader::parser::MarkdownParser {
            state: panduck_types::parser::ParserState::new(&source, tokens),
            config: &self.config,
        };
        let PanduckDiagnostics {
            result,
            mut diagnostics,
        } = parser.parse();
        errors.append(&mut diagnostics);
        match result {
            Ok(tokens) => PanduckDiagnostics {
                result: Ok(tokens),
                diagnostics: errors,
            },
            Err(e) => PanduckDiagnostics {
                result: Err(e),
                diagnostics: errors,
            },
        }
    }
}
