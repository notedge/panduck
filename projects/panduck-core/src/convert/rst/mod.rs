mod body;
mod inline;

use crate::{ExtensionHandler, ExtensionRegistrar, Result, ToNotedown};
use document_tree::{
    element_categories::{BodyElement, StructuralSubElement, SubStructure, TextOrInlineElement},
    Decoration, Docinfo, Document, HasChildren, Inline, Math, Section, Subtitle, Title,
};
use notedown_ast::{ASTKind, ASTNode, ASTNodes};
use std::{collections::BTreeSet, iter::FromIterator};

pub fn register_rst(r: &mut ExtensionRegistrar) {
    let ext = vec!["rst"];
    let new = ExtensionHandler {
        name: String::from("reStructuredText"),
        try_extension: BTreeSet::from_iter(ext.into_iter().map(String::from)),
        parser: parse_rst,
    };
    r.insert(new)
}

pub fn parse_rst(input: &str) -> Result<ASTNode> {
    let parsed = match rst_parser::parse(input) {
        Ok(o) => o,
        Err(e) => {
            unimplemented!("{:#?}", e)
        }
    };
    return Ok(parsed.into_notedown());
}

impl ToNotedown for Document {
    fn into_notedown(self) -> ASTNode {
        ASTKind::statements(self.into_notedown_list(), None)
    }

    fn into_notedown_list(self) -> ASTNodes {
        self.children().iter().cloned().map(|e| e.into_notedown()).collect()
    }
}

impl ToNotedown for Vec<StructuralSubElement> {
    fn into_notedown(self) -> ASTNode {
        ASTKind::paragraph(self.into_notedown_list(), None)
    }

    fn into_notedown_list(self) -> ASTNodes {
        self.into_iter().map(|e| e.into_notedown()).collect()
    }
}

impl ToNotedown for StructuralSubElement {
    fn into_notedown(self) -> ASTNode {
        match self {
            Self::Title(v) => ASTKind::header(v.children().clone().into_notedown_list(), 1, None),
            Self::Subtitle(v) => ASTKind::header(v.children().clone().into_notedown_list(), 2, None),
            Self::Decoration(v) => v.into_notedown(),
            Self::Docinfo(v) => v.into_notedown(),
            Self::SubStructure(v) => v.into_notedown(),
        }
    }
}

impl ToNotedown for Decoration {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }
}
impl ToNotedown for Docinfo {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }
}

impl ToNotedown for SubStructure {
    fn into_notedown(self) -> ASTNode {
        match self {
            Self::Topic(v) => {
                todo!("{:#?}", v)
            }
            Self::Sidebar(v) => {
                todo!("{:#?}", v)
            }
            Self::Transition(v) => {
                todo!("{:#?}", v)
            }
            Self::Section(v) => v.into_notedown(),
            Self::BodyElement(v) => v.into_notedown(),
        }
    }
}

impl ToNotedown for Section {
    fn into_notedown(self) -> ASTNode {
        self.children().clone().into_notedown()
    }
}

impl ToNotedown for Math {
    fn into_notedown(self) -> ASTNode {
        let _t = self.children().join("");
        todo!()
    }
}
