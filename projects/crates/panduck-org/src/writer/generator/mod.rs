#![doc = include_str!("readme.md")]

use crate::ast::{OrgBlock, OrgBlockCode, OrgHeading, OrgInline, OrgList, OrgParagraph, OrgRoot};
use crate::writer::OrgWriteConfig;
use panduck_types::{PanduckDiagnostics, PanduckError, TextWriter};
use panduck_markdown::ast::MarkdownInline;
use panduck_markdown::writer::{MarkdownWriter, MarkdownWriteConfig};
use std::fmt::Write;

#[derive(Debug)]
pub struct OrgWriter<'input, W> {
    writer: TextWriter<W>,
    config: &'input OrgWriteConfig,
    markdown_writer: MarkdownWriter<'input, W>,
}

impl OrgWriteConfig {
    pub fn writer<W: Write + Copy>(&self, writer: W) -> OrgWriter<W> {
        OrgWriter {
            writer: TextWriter::new(writer),
            config: self,
            markdown_writer: MarkdownWriter::new(writer, &self.markdown_config),
        }
    }
}

impl<'input, W: Write> OrgWriter<'input, W> {
    pub fn generate(mut self, ast: &OrgRoot) -> PanduckDiagnostics<W> {
        let mut errors = Vec::new();
        for block in &ast.blocks {
            match self.write_block(block) {
                Ok(o) => {}
                Err(e) => errors.push(e),
            }
        }
        PanduckDiagnostics {
            result: Ok(self.writer.finish()),
            diagnostics: errors,
        }
    }

    fn write_block(&mut self, block: &OrgBlock) -> Result<(), PanduckError> {
        match block {
            OrgBlock::Heading(heading) => self.write_heading(heading),
            OrgBlock::Paragraph(paragraph) => self.write_paragraph(paragraph),
            OrgBlock::BlockCode(block_code) => self.write_block_code(block_code),
            OrgBlock::List(list) => self.write_list(list),
        }
    }

    fn write_heading(&mut self, heading: &OrgHeading) -> Result<(), PanduckError> {
        for _ in 0..heading.level {
            self.writer.write("#")?;
        }
        self.writer.write(" ")?;
        self.markdown_writer.write_inlines(&Self::convert_org_inlines_to_markdown_inlines(&heading.content))?;
        self.writer.write_line("")?;
        self.writer.write_line("")
    }

    fn write_paragraph(&mut self, paragraph: &OrgParagraph) -> Result<(), PanduckError> {
        self.markdown_writer.write_inlines(&Self::convert_org_inlines_to_markdown_inlines(&paragraph.content))?;
        self.writer.write_line("")?;
        self.writer.write_line("")
    }
    fn write_block_code(&mut self, block_code: &OrgBlockCode) -> Result<(), PanduckError> {
        self.writer
            .write_line(&format!("```{}", block_code.language.unwrap_or_default()))?;
        self.writer.write_line(&block_code.content)?;
        self.writer.write_line("```")?;
        self.writer.write_line("")
    }

    fn write_list(&mut self, list: &OrgList) -> Result<(), PanduckError> {
        self.writer.write_line("")?;
        match list {
            OrgList::Unordered(items) => {
                for item in items {
                    self.writer.write("    ")?;
                    self.writer.write("- ")?;
                    self.markdown_writer.write_inlines(&Self::convert_org_inlines_to_markdown_inlines(&item.content))?;
                    self.writer.write_line("")?;
                }
            }
            OrgList::Ordered(items) => {
                for (i, item) in items.into_iter().enumerate() {
                    self.writer.write("    ")?;
                    self.writer.write(&format!("{}. ", i + 1))?;
                    self.markdown_writer.write_inlines(&Self::convert_org_inlines_to_markdown_inlines(&item.content))?;
                    self.writer.write_line("")?;
                }
            }
        }
        self.writer.write_line("")
    }

    fn convert_org_inlines_to_markdown_inlines(inlines: &Vec<OrgInline>) -> Vec<MarkdownInline> {
        let mut markdown_inlines = Vec::new();
        for inline in inlines {
            match inline {
                OrgInline::Text(text) => markdown_inlines.push(MarkdownInline::Text(text.clone())),
                OrgInline::Bold(inlines) => markdown_inlines.push(MarkdownInline::Bold(Self::convert_org_inlines_to_markdown_inlines(inlines))),
                OrgInline::Italic(inlines) => markdown_inlines.push(MarkdownInline::Italic(Self::convert_org_inlines_to_markdown_inlines(inlines))),
                OrgInline::Code(code) => markdown_inlines.push(MarkdownInline::Code(code.clone())),
                OrgInline::Link(text, url) => markdown_inlines.push(MarkdownInline::Link(Self::convert_org_inlines_to_markdown_inlines(text), url.clone())),
                OrgInline::Image(alt_text, url) => markdown_inlines.push(MarkdownInline::Image(Self::convert_org_inlines_to_markdown_inlines(alt_text), url.clone())),
            }
        }
        markdown_inlines
    }
}