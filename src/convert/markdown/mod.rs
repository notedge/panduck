use crate::{ToNotedown, AST};
use markdown::{tokenize, Block, ListItem, Span};

impl ToNotedown for Vec<Block> {
    fn to_notedown(&self) -> AST {
        AST::Statements(self.iter().map(ToNotedown::to_notedown).collect())
    }
}

impl ToNotedown for Block {
    fn to_notedown(&self) -> AST {
        match self {
            Block::Header(_, _) => unimplemented!(),
            Block::Paragraph(_) => unimplemented!(),
            Block::Blockquote(_) => unimplemented!(),
            Block::CodeBlock(_, _) => unimplemented!(),
            Block::OrderedList(_, _) => unimplemented!(),
            Block::UnorderedList(_) => unimplemented!(),
            Block::Raw(_) => unimplemented!(),
            Block::Hr => unimplemented!(),
        }
    }
}

impl ToNotedown for Vec<Span> {
    fn to_notedown(&self) -> AST {
        let mut out = vec![];
        for node in self {
            let ast = node.to_notedown();
            match ast {
                AST::None => continue,
                AST::Statements(v) => out.extend(v),
                _ => out.push(ast),
            }
        }
        return AST::Statements(out);
    }
}

impl ToNotedown for Span {
    fn to_notedown(&self) -> AST {
        match self {
            Span::Break => unimplemented!(),
            Span::Text(_) => unimplemented!(),
            Span::Code(_) => unimplemented!(),
            Span::Link(_, _, _) => unimplemented!(),
            Span::Image(_, _, _) => unimplemented!(),
            Span::Emphasis(_) => unimplemented!(),
            Span::Strong(_) => unimplemented!(),
        }
    }
}

// impl From<Vec<Block>> for AST {
// fn from(v: Vec<Block>) -> Self {
// AST::Statements(v.into_iter().map(Into::into).collect())
// }
// }
//
// impl From<Block> for AST {
// fn from(v: Block) -> Self {
// match v {
// Block::Header(content, level) => AST::Header(content.into_iter().map(Into::into).collect(), level),
// Block::Paragraph(p) => p.into(),
// Block::CodeBlock(lang, code) => {
// let lang = match lang {
// None => "txt",
// Some(s) => match s.as_ref() {
// "" => "txt",
// lang tokens created from the String would be available across all threads
// _ => Box::leak(s.into_boxed_str()),
// },
// };
// let code = Highlighter { lang: lang.to_string(), code: code.into(), inline: false, high_line: vec![] };
// AST::Highlight(code)
// }
// Block::Blockquote(q) => {
// let list = ListView::Quote { style: None, body: q.into_iter().map(Into::into).collect() };
// AST::List(list)
// }
// Block::OrderedList(o, _) => {
// let list = ListView::Ordered { head: 1, body: o.into_iter().map(Into::into).collect() };
// AST::List(list)
// }
// Block::UnorderedList(u) => {
// let list = ListView::Orderless { body: u.into_iter().map(Into::into).collect() };
// AST::List(list)
// }
// Block::Raw(_) => unimplemented!(),
// Block::Hr => unimplemented!(),
// }
// }
// }
//
// impl From<Vec<Span>> for AST {
// fn from(v: Vec<Span>) -> Self {
// AST::Paragraph(v.into_iter().map(Into::into).collect())
// }
// }
//
// impl From<Span> for AST {
// fn from(v: Span) -> Self {
// match v {
// Span::Break => unimplemented!(),
// Span::Text(text) => AST::Normal(text.into()),
// Span::Code(c) => AST::Code(c.into()),
// Span::Link(text, url, title) => {
// let link = SmartLink::Hyperlinks { from: text.into(), to: url.into(), alt: title.map(Into::into), bind: None };
// AST::Link(link)
// }
// Span::Image(_, src, title) => {
// let link = SmartLink::Image { img: src.into(), to: None, alt: title.map(Into::into), bind: None };
// AST::Link(link)
// }
// Span::Emphasis(e) => AST::Emphasis(e.into_iter().map(Into::into).collect()),
// Span::Strong(s) => AST::Strong(s.into_iter().map(Into::into).collect()),
// }
// }
// }
//
// impl From<ListItem> for AST {
// fn from(v: ListItem) -> Self {
// match v {
// ListItem::Simple(s) => s.into(),
// ListItem::Paragraph(p) => p.into(),
// }
// }
// }
