use crate::{SmartLink, ToNotedown, };
use markdown::{Block, ListItem, Span};
use notedown_parser::ASTKind;
use crate::convert::AST;


impl ToNotedown for Vec<Block> {
    fn to_notedown(&self) -> AST {
        ASTKind::statements(self.iter().map(ToNotedown::to_notedown).collect(), ).into()
    }
}

impl ToNotedown for Block {
    fn to_notedown(&self) -> AST {
        match self {
            Block::Header(content, level) => AST::header( content.to_notedown().to_vec(), *level, ),
            Block::Paragraph(p) => AST::paragraph(p.to_notedown().to_vec(),),
            Block::CodeBlock(_, _) => unimplemented!(),
            Block::Raw(_) => unimplemented!(),
            Block::Hr => unimplemented!(),
            // Block::Blockquote(list) => AST::QuoteList { style: None, body: list.to_notedown().to_vec(), r },
            // Block::OrderedList(list, _) => AST::OrderedList { head: 1, body: list.to_notedown().to_vec(), r },
            // Block::UnorderedList(list) => AST::OrderlessList { body: list.to_notedown().to_vec(), r },
            Block::Blockquote(_) => unimplemented!(),
            Block::OrderedList(_, _) => unimplemented!(),
            Block::UnorderedList(_) => unimplemented!(),
        }
    }
}

impl ToNotedown for Vec<Span> {
    fn to_notedown(&self) -> AST {
        let mut out = vec![];
        for node in self {
            let ast = node.to_notedown();
            match ast.kind {
                ASTKind::None => continue,
                ASTKind::Statements(v) => out.extend(v),
                _ => out.push(ast),
            }
        }
        return AST::statements(out, );
    }
}

impl ToNotedown for Span {
    fn to_notedown(&self) -> AST {
        let r = Default::default();
        match self {
            Span::Break => unimplemented!(),
            Span::Text(t) => AST::text(t.to_owned(), "text", ),
            Span::Code(code) => {
                AST::Highlight { lang: String::from("txt"), code: code.to_owned(), inline: true, high_line: vec![], r }
            }
            Span::Link(text, url, title) => {
                let link = SmartLink::Hyperlinks {
                    from: text.into(),
                    to: Some(url.into()),
                    alt: title.as_ref().map(Into::into),
                    bind: None,
                };
                AST::Link { inner: link, r }
            }
            Span::Image(_, _, _) => unimplemented!(),
            Span::Emphasis(_) => unimplemented!(),
            Span::Strong(_) => unimplemented!(),
        }
    }
}

impl ToNotedown for Vec<ListItem> {
    fn to_notedown(&self) -> AST {
        AST::statements(self.iter().map(ToNotedown::to_notedown).collect(), )
    }
}

impl ToNotedown for ListItem {
    fn to_notedown(&self) -> AST {
        match self {
            ListItem::Simple(s) => s.to_notedown(),
            ListItem::Paragraph(p) => AST::paragraph(p.to_notedown().to_vec(),),
        }
    }
}
