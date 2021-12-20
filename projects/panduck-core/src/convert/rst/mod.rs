mod inline;

use crate::ToNotedown;
use document_tree::{
    element_categories::{StructuralSubElement, SubStructure, TextOrInlineElement},
    Decoration, Docinfo, Document, HasChildren, Inline, Math, Subtitle, Title,
};
use notedown_ast::{ASTKind, ASTNode, ASTNodes};

impl ToNotedown for Document {
    fn into_notedown(self) -> ASTNode {
        ASTKind::statements(self.into_notedown_list(), None)
    }

    fn into_notedown_list(self) -> ASTNodes {
        self.children().iter().cloned().map(|e| e.into_notedown()).collect()
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
        todo!()
    }
}

impl ToNotedown for Math {
    fn into_notedown(self) -> ASTNode {
        let _t = self.children().join("");
        todo!()
    }
}
