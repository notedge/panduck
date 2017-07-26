#![doc = include_str!("readme.md")]

use crate::ast::{
    MarkdownBlock, MarkdownBlockCode, MarkdownHeading, MarkdownInline, MarkdownList,
    MarkdownParagraph, MarkdownRoot, MarkdownText,
};
use crate::writer::MarkdownWriteConfig;
use panduck_types::{PanduckDiagnostics, PanduckError, TextWriter};
use std::fmt::Write;
// Add this import

#[derive(Debug)]
pub struct MarkdownWriter<'input, W> {
    writer: TextWriter<W>,
    config: &'input MarkdownWriteConfig,
}

impl MarkdownWriteConfig {
    pub fn writer<W>(&self, writer: W) -> MarkdownWriter<W> {
        MarkdownWriter {
            writer: TextWriter::new(writer),
            config: &self,
        }
    }
}

impl<'input, W: Write> MarkdownWriter<'input, W> {
    pub fn generate(mut self, ast: &MarkdownRoot) -> PanduckDiagnostics<W> {
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

    fn write_block(&mut self, block: &MarkdownBlock) -> Result<(), PanduckError> {
        match block {
            MarkdownBlock::Heading(heading) => self.write_heading(heading),
            MarkdownBlock::Paragraph(paragraph) => self.write_paragraph(paragraph),
            MarkdownBlock::BlockCode(block_code) => self.write_block_code(block_code),
            MarkdownBlock::List(list) => self.write_list(list),
        }
    }

    fn write_heading(&mut self, heading: &MarkdownHeading) -> Result<(), PanduckError> {
        for _ in 0..heading.level {
            self.writer.write("#")?;
        }
        self.writer.write(" ")?;
        self.write_inlines(&heading.content)?;
        self.writer.write_line("")?;
        self.writer.write_line("")?;
        Ok(())
    }

    fn write_paragraph(&mut self, paragraph: &MarkdownParagraph) -> Result<(), PanduckError> {
        self.write_inlines(&paragraph.content)?;
        self.writer.write_line("")?;
        self.writer.write_line("")?;
        Ok(())
    }
    fn write_block_code(&mut self, block_code: &MarkdownBlockCode) -> Result<(), PanduckError> {
        self.writer.write("```")?;
        self.writer.write_line(&block_code.language)?;
        self.writer.write_line(&block_code.content)?;
        self.writer.write_line("```")?;
        self.writer.write_line("")?;
        Ok(())
    }

    fn write_list(&mut self, list: &MarkdownList) -> Result<(), PanduckError> {
        self.writer.write_line("")?;
        match list {
            MarkdownList::Unordered(items) => {
                for item in items {
                    self.writer.write("    ")?;
                    self.writer.write("- ")?;
                    self.write_inlines(&item.content)?;
                    self.writer.write_line("")?;
                }
            }
            MarkdownList::Ordered(items) => {
                for (i, item) in items.into_iter().enumerate() {
                    self.writer.write("    ")?;
                    self.writer.write(&format!("{}. ", i + 1))?;
                    self.write_inlines(&item.content)?;
                    self.writer.write_line("")?;
                }
            }
        }
        self.writer.write_line("")?;
        Ok(())
    }

    fn write_inlines(&mut self, inlines: &[MarkdownInline]) -> Result<(), PanduckError> {
        for inline in inlines {
            match inline {
                MarkdownInline::Text(MarkdownText { text: text }) => self.writer.write(&text)?,
                MarkdownInline::Bold(bold) => {
                    self.writer.write("**")?;
                    self.write_inlines(&bold.contents)?;
                    self.writer.write("**")?;
                }
                MarkdownInline::Italic(italic) => {
                    self.writer.write("*")?;
                    self.write_inlines(&italic.contents)?;
                    self.writer.write("*")?;
                }
                MarkdownInline::Code(code) => {
                    self.writer.write("`")?;
                    self.writer.write(&code)?;
                    self.writer.write("`")?;
                }
                MarkdownInline::Link(link) => {
                    self.writer.write("[")?;
                    self.writer.write(&link.alt)?;
                    self.writer.write("](")?;
                    self.writer.write(&link.url)?;
                    self.writer.write(")")?;
                }
                MarkdownInline::Image(image) => {
                    self.writer.write("![")?;
                    self.writer.write(&image.alt)?;
                    self.writer.write("](")?;
                    self.writer.write(&image.url)?;
                    self.writer.write(")")?;
                }
            }
        }
        Ok(())
    }
}
