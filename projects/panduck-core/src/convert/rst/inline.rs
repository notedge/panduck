use super::*;

impl ToNotedown for Vec<TextOrInlineElement> {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }

    fn into_notedown_list(self) -> ASTNodes {
        todo!()
    }
}

impl ToNotedown for TextOrInlineElement {
    fn into_notedown(self) -> ASTNode {
        match self {
            TextOrInlineElement::String(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Emphasis(v) => ASTKind::emphasis(v.children().clone().into_notedown_list(), None),
            TextOrInlineElement::Strong(v) => ASTKind::strong(v.children().clone().into_notedown_list(), None),
            TextOrInlineElement::Literal(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Reference(_) => {
                unimplemented!()
            }
            TextOrInlineElement::FootnoteReference(_) => {
                unimplemented!()
            }
            TextOrInlineElement::CitationReference(_) => {
                unimplemented!()
            }
            TextOrInlineElement::SubstitutionReference(_) => {
                unimplemented!()
            }
            TextOrInlineElement::TitleReference(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Abbreviation(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Acronym(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Superscript(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Subscript(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Inline(v) => v.into_notedown(),
            TextOrInlineElement::Problematic(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Generated(_) => {
                unimplemented!()
            }
            TextOrInlineElement::Math(v) => v.into_notedown(),
            TextOrInlineElement::TargetInline(_) => {
                unimplemented!()
            }
            TextOrInlineElement::RawInline(_) => {
                unimplemented!()
            }
            TextOrInlineElement::ImageInline(_) => {
                unimplemented!()
            }
        }
    }
}

impl ToNotedown for Inline {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }
}
