mod lexer;
mod parser;
mod token_type;

pub use self::token_type::{OrgToken, OrgTokenType};
pub use crate::ast::OrgRoot;
use crate::reader::lexer::OrgLexer;
use panduck_types::helpers::{check_path, SourceText};
use panduck_types::{AdapterError, Result};
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

    pub fn read_str(&self, text: &str, url: Option<Url>) -> Result<OrgRoot> {
        self.reader(text.as_bytes()).read(url)
    }

    pub fn read_path(&self, path: impl AsRef<Path>) -> Result<OrgRoot> {
        let (file, url) = check_path(path)?;
        self.reader(file).read(Some(url))
    }
}

impl<'input, R: Read> OrgReader<'input, R> {
    pub fn read(mut self, url: Option<Url>) -> Result<OrgRoot> {
        let text = read_to_string(&mut self.reader).map_err(AdapterError::from)?;
        let source = SourceText::new(text, url);
        let lexer = OrgLexer { config: &self.config };
        let tokens = lexer.tokenize()?;
        let mut parser = crate::reader::parser::OrgParser {
            config: &self.config,
        };
        let _ = (source, tokens);
        parser.parse()
    }
}
