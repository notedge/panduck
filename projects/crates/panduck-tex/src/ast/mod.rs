#![doc = include_str!("readme.md")]

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownRoot {
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MarkdownBlock {
    Heading(MarkdownHeading),
    Paragraph(MarkdownParagraph),
    BlockCode(MarkdownBlockCode),
    List(MarkdownList),
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownHeading {
    pub level: u8,
    pub content: Vec<MarkdownInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownParagraph {
    pub content: Vec<MarkdownInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownBlockCode {
    pub language: Option<String>,
    pub content: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MarkdownList {
    Unordered(Vec<MarkdownListItem>),
    Ordered(Vec<MarkdownListItem>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownListItem {
    pub content: Vec<MarkdownInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MarkdownInline {
    Text(String),
    Bold(Vec<MarkdownInline>),
    Italic(Vec<MarkdownInline>),
    Code(String),
    Link(String, String), // text, url
    Image(String, String), // alt_text, url
}
