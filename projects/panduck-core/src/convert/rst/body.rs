use document_tree::{BulletList, Comment, ExtraAttributes, Image, LiteralBlock, Note, Paragraph, Target, Warning};
use notedown_ast::{nodes::ImageLink, traits::IntoASTNode};

use super::*;

impl ToNotedown for BodyElement {
    fn into_notedown(self) -> ASTNode {
        match self {
            Self::Paragraph(v) => v.into_notedown(),
            Self::LiteralBlock(v) => v.into_notedown(),
            Self::DoctestBlock(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::MathBlock(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Rubric(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::SubstitutionDefinition(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Comment(v) => v.into_notedown(),
            Self::Pending(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Target(v) => v.into_notedown(),
            Self::Raw(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Image(v) => v.into_notedown(),
            Self::Compound(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Container(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::BulletList(v) => v.into_notedown(),
            Self::EnumeratedList(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::DefinitionList(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::FieldList(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::OptionList(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::LineBlock(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::BlockQuote(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Admonition(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Attention(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Hint(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Note(v) => v.into_notedown(),
            Self::Caution(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Danger(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Error(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Important(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Tip(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Warning(v) => v.into_notedown(),
            Self::Footnote(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Citation(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::SystemMessage(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Figure(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Table(v) => {
                unimplemented!("{:#?}", v)
            }
        }
    }
}

impl ToNotedown for Target {
    fn into_notedown(self) -> ASTNode {
        // todo
        ASTNode::default()
    }
}

impl ToNotedown for Paragraph {
    fn into_notedown(self) -> ASTNode {
        ASTKind::paragraph(self.children().clone().into_notedown_list(), None)
    }
}

impl ToNotedown for Comment {
    fn into_notedown(self) -> ASTNode {
        // todo
        ASTNode::default()
    }
}

impl ToNotedown for LiteralBlock {
    fn into_notedown(self) -> ASTNode {
        ASTKind::paragraph(self.children().clone().into_notedown_list(), None)
    }
}

impl ToNotedown for Warning {
    fn into_notedown(self) -> ASTNode {
        // todo
        ASTNode::default()
    }
}

impl ToNotedown for Note {
    fn into_notedown(self) -> ASTNode {
        // todo
        ASTNode::default()
    }
}

impl ToNotedown for Image {
    fn into_notedown(self) -> ASTNode {
        let image = self.extra();
        let size = Default::default();
        // let a = image.height;
        ImageLink {
            source: image.uri.to_string(),
            description: image.alt.to_owned(),
            link: None,
            force_caption: None,
            layout: None,
            size: Some(size),
            options: None,
        }
        .into_node(None)
    }
}

impl ToNotedown for BulletList {
    fn into_notedown(self) -> ASTNode {
        // todo
        ASTNode::default()
    }
}
