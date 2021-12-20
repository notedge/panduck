use super::*;
use document_tree::{Comment, LiteralBlock, Paragraph, Target, Warning};

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
            Self::Image(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Compound(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::Container(v) => {
                unimplemented!("{:#?}", v)
            }
            Self::BulletList(v) => {
                unimplemented!("{:#?}", v)
            }
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
            Self::Note(v) => {
                unimplemented!("{:#?}", v)
            }
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
