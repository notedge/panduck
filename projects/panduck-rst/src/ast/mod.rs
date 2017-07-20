//! Abstract Syntax Tree (AST) for panduck-rst.

#[derive(Debug, PartialEq, Clone)]
pub struct RstRoot {
    pub blocks: Vec<RstBlock>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RstBlock {
    Heading(RstHeading),
    Paragraph(RstParagraph),
    BlockCode(RstBlockCode),
    List(RstList),
}

#[derive(Debug, PartialEq, Clone)]
pub struct RstHeading {
    pub level: u8,
    pub content: Vec<RstInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RstParagraph {
    pub content: Vec<RstInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RstBlockCode {
    pub lang: Option<String>,
    pub content: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RstList {
    pub items: Vec<RstListItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RstListItem {
    pub content: Vec<RstInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RstInline {
    Text(String),
    Bold(Vec<RstInline>),
    Italic(Vec<RstInline>),
    Code(String),
    Link(String, String),
    Image(String, String),
}
