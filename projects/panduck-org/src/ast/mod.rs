#![doc = include_str!("readme.md")]

#[derive(Debug, PartialEq, Clone)]
pub struct OrgRoot {
    pub blocks: Vec<OrgBlock>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrgBlock {
    Heading(OrgHeading),
    Paragraph(OrgParagraph),
    BlockCode(OrgBlockCode),
    List(OrgList),
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrgHeading {
    pub level: u8,
    pub content: Vec<OrgInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrgParagraph {
    pub content: Vec<OrgInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrgBlockCode {
    pub language: Option<String>,
    pub content: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrgList {
    Unordered(Vec<OrgListItem>),
    Ordered(Vec<OrgListItem>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrgListItem {
    pub content: Vec<OrgInline>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrgInline {
    Text(String),
    Bold(Vec<OrgInline>),
    Italic(Vec<OrgInline>),
    Code(String),
    Link(Vec<OrgInline>, String), // text, url
    Image(Vec<OrgInline>, String), // alt_text, url
}
