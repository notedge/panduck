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
    pub language: String,
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
    Text(MarkdownText),
    Bold(MarkdownBold),
    Italic(MarkdownItalic),
    Code(String),
    Link(MarkdownLink),
    Image(MarkdownImage),
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownBold {
    pub contents: Vec<MarkdownInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownItalic {
    pub contents: Vec<MarkdownInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownImage {
    pub alt: String,
    pub url: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownLink {
    pub alt: String,
    pub url: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MarkdownText {
    pub text: String,
}
