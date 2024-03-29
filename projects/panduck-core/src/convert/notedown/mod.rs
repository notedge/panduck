use super::*;

use notedown_parser::{utils::LSPMetaInfo, ASTKind, ASTNode, ParserConfig};

pub fn register_notedown(r: &mut ExtensionRegistrar) {
    let ext = vec!["note"];
    let parser = |input| Ok(ParserConfig::default().parse(input)?.to_notedown());
    let new = ExtensionHandler { try_extension: BTreeSet::from_iter(ext.iter().map(String::from)), parser };
    r.insert(new)
}

impl ToNotedown for ASTNode<LSPMetaInfo> {
    fn into_notedown(&self) -> AST {
        self.kind.to_notedown()
    }
}

impl ToNotedown for ASTKind<ASTNode<LSPMetaInfo>> {
    fn into_notedown(&self) -> AST {
        match self {
            ASTKind::None => AST { kind: ASTKind::None, meta: () },
            ASTKind::Statements(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Header(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::HorizontalRule => AST { kind: ASTKind::None, meta: () },
            ASTKind::Paragraph(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::CodeBlock(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::MathBlock(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::TableView(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::ListView(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Normal(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Raw(string) => AST { kind: ASTKind::Raw(string.to_owned()), meta: () },
            ASTKind::Code(string) => AST { kind: ASTKind::Code(string.to_owned()), meta: () },
            ASTKind::Italic(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Bold(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Emphasis(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Underline(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Strikethrough(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Undercover(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::MathInline(string) => AST { kind: ASTKind::MathInline(string.to_owned()), meta: () },
            ASTKind::MathDisplay(string) => AST { kind: ASTKind::MathDisplay(string.to_owned()), meta: () },
            ASTKind::Escaped(e) => AST { kind: ASTKind::Escaped(*e), meta: () },
            ASTKind::Link(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Command(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::String(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Number(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Boolean(_) => AST { kind: ASTKind::None, meta: () },
            ASTKind::Array => AST { kind: ASTKind::None, meta: () },
            ASTKind::Object => AST { kind: ASTKind::None, meta: () },
        }
    }
}
