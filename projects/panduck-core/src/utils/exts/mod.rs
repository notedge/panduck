use crate::{PanduckError, Result};
use notedown_ast::ASTNode;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs::read_to_string,
    path::Path,
    rc::Rc,
};

pub struct ExtensionRegistrar {
    arena: Vec<Rc<ExtensionHandler>>,
    inner: BTreeMap<String, BTreeSet<Rc<ExtensionHandler>>>,
}

pub struct ExtensionHandler {
    pub name: String,
    pub parser: fn(&str) -> Result<ASTNode>,
    pub try_extension: BTreeSet<String>,
}

impl ExtensionRegistrar {
    pub fn insert(&mut self, h: ExtensionHandler) {
        let handler = Rc::new(h);
        self.arena.push(Rc::clone(&handler));
        for i in &handler.try_extension {
            let new = Rc::clone(&handler);
            let ptr = self.inner.get_mut(i.as_str());
            match ptr {
                Some(s) => {
                    s.insert(new);
                }
                None => {
                    let mut s = BTreeSet::new();
                    s.insert(new);
                    self.inner.insert(i.to_owned(), s);
                }
            }
        }
    }
    #[inline]
    pub fn get(&self, ext: &str) -> BTreeSet<Rc<ExtensionHandler>> {
        // self.inner.get(ext).unwrap_or_default()
        match self.inner.get(ext) {
            Some(s) => s.to_owned(),
            None => BTreeSet::new(),
        }
    }
    pub fn parse_by_ext(&self, file: impl AsRef<Path>) -> Result<ASTNode> {
        let ext = file.as_ref().extension().and_then(|t| t.to_str()).ok_or(())?;
        let input = &read_to_string(file.as_ref())?;
        for t in self.get(ext) {
            let parse = t.parser;
            match parse(input) {
                Ok(s) => return Ok(s),
                Err(_) => continue,
            }
        }
        let mut error = PanduckError::unsupported_file(ext);
        error.set_path(file);
        return Err(error);
    }
}

impl Eq for ExtensionHandler {}

impl PartialEq<Self> for ExtensionHandler {
    fn eq(&self, other: &ExtensionHandler) -> bool {
        self.name.eq(&other.name)
    }
}

impl PartialOrd<Self> for ExtensionHandler {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for ExtensionHandler {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
