mod lexer;
mod parser;
mod token_type;

pub use self::token_type::{MarkdownToken, MarkdownTokenType};
pub use crate::ast::MarkdownRoot;
use crate::reader::lexer::MarkdownLexer;
use panduck_types::helpers::{check_path, SourceText};
use panduck_types::{AdapterError, Result};
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

    pub fn read_str(&self, text: &str, url: Option<Url>) -> Result<MarkdownRoot> {
        self.reader(text.as_bytes()).read(url)
    }

    pub fn read_path(&self, path: impl AsRef<Path>) -> Result<MarkdownRoot> {
        let (file, url) = check_path(path)?;
        self.reader(file).read(Some(url))
    }
}

impl<'input, R: Read> MarkdownReader<'input, R> {
    pub fn read(mut self, url: Option<Url>) -> Result<MarkdownRoot> {
        let text = read_to_string(&mut self.reader).map_err(AdapterError::from)?;
        let source = SourceText::new(text, url);
        let mut lexer = MarkdownLexer {
            state: panduck_types::lexer::LexerState::new(&source),
            config: &self.config,
        };
        let tokens = lexer.tokenize()?;
        let mut parser = crate::reader::parser::MarkdownParser {
            state: panduck_types::parser::ParserState::new(&source, tokens),
            config: &self.config,
        };
        parser.parse()
    }
}
