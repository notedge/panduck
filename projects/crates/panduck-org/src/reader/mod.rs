mod lexer;
mod parser;
mod token_type;

pub use self::token_type::{OrgToken, OrgTokenType};
pub use crate::ast::OrgRoot;
use crate::reader::lexer::OrgLexer;
use panduck_core::helpers::{check_path, SourceText};
use panduck_core::{PanduckDiagnostics, PanduckError};
use std::io::{read_to_string, Read};
use std::path::Path;
use url::Url;

#[derive(Copy, Clone, Debug)]
pub struct OrgReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct OrgReader<'input, R> {
    reader: R,
    config: &'input OrgReadConfig,
}

impl OrgReadConfig {
    pub fn reader<R>(&self, reader: R) -> OrgReader<R> {
        OrgReader {
            reader,
            config: self,
        }
    }
    pub fn read_str(&self, text: &str, url: Option<Url>) -> PanduckDiagnostics<OrgRoot> {
        self.reader(text.as_bytes()).read(url)
    }

    pub fn read_path(&self, path: impl AsRef<Path>) -> PanduckDiagnostics<OrgRoot> {
        match check_path(path) {
            Ok((file, url)) => self.reader(file).read(Some(url)),
            Err(e) => PanduckDiagnostics {
                result: Err(e),
                diagnostics: vec![],
            },
        }
    }
}

impl<'input, R: Read> OrgReader<'input, R> {
    pub fn read(mut self, url: Option<Url>) -> PanduckDiagnostics<OrgRoot> {
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
        let mut lexer = OrgLexer {
            state: panduck_core::lexer::LexerState::new(&source),
            config: &self.config,
        };
        let PanduckDiagnostics {
            result,
            mut diagnostics,
        } = lexer.tokenize();
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
        let mut parser = crate::reader::parser::OrgParser {
            state: panduck_core::parser::ParserState::new(&source, tokens),
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
