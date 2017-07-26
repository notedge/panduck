use crate::ast::{RstBlock, RstBlockCode, RstHeading, RstInline, RstList, RstListItem, RstParagraph, RstRoot};
use panduck_types::generator::{Generator, GeneratorConfig};
use panduck_types::PanduckDiagnostics;

pub struct RstGeneratorConfig;

impl GeneratorConfig for RstGeneratorConfig {
    type Block = RstBlock;
    type Inline = RstInline;
    type Root = RstRoot;
}

pub struct GeneratorState {
    output: String,
    diagnostics: PanduckDiagnostics<String>,
}

impl Generator for GeneratorState {
    type Config = RstGeneratorConfig;

    fn new() -> Self {
        Self {
            output: String::new(),
            diagnostics: PanduckDiagnostics::new(),
        }
    }

    fn diagnostics(&self) -> &PanduckDiagnostics<String> {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut PanduckDiagnostics<String> {
        &mut self.diagnostics
    }

    fn generate_root(&mut self, root: RstRoot) -> String {
        for block in root.blocks {
            self.generate_block(block);
        }
        self.output.clone()
    }

    fn generate_block(&mut self, block: RstBlock) {
        match block {
            RstBlock::Heading(heading) => self.generate_heading(heading),
            RstBlock::Paragraph(paragraph) => self.generate_paragraph(paragraph),
            RstBlock::BlockCode(block_code) => self.generate_block_code(block_code),
            RstBlock::List(list) => self.generate_list(list),
        }
    }

    fn generate_inline(&mut self, inline: RstInline) {
        match inline {
            RstInline::Text(text) => self.output.push_str(&text),
            RstInline::Bold(content) => {
                self.output.push_str("**");
                for item in content {
                    self.generate_inline(item);
                }
                self.output.push_str("**");
            }
            RstInline::Italic(content) => {
                self.output.push_str("*");
                for item in content {
                    self.generate_inline(item);
                }
                self.output.push_str("*");
            }
            RstInline::Code(code) => {
                self.output.push_str("``");
                self.output.push_str(&code);
                self.output.push_str("``");
            }
            RstInline::Link { text, url } => {
                self.output.push_str("`");
                for item in text {
                    self.generate_inline(item);
                }
                self.output.push_str(&format!(" <{}>`_", url));
            }
            RstInline::Image { alt, url } => {
                self.output.push_str(&format!(".. image:: {}\n   :alt: {}\n", url, alt));
            }
        }
    }
}

impl GeneratorState {
    fn generate_heading(&mut self, heading: RstHeading) {
        let underline = "=".repeat(heading.content.iter().map(|inline| {
            match inline {
                RstInline::Text(text) => text.len(),
                _ => 0, // For simplicity, assuming heading content is mostly text for underline calculation
            }
        }).sum());
        for inline in heading.content {
            self.generate_inline(inline);
        }
        self.output.push_str(&format!("\n{}\n", underline));
    }

    fn generate_paragraph(&mut self, paragraph: RstParagraph) {
        for inline in paragraph.content {
            self.generate_inline(inline);
        }
        self.output.push_str("\n\n");
    }

    fn generate_block_code(&mut self, block_code: RstBlockCode) {
        self.output.push_str("::\n\n");
        for line in block_code.code.lines() {
            self.output.push_str(&format!("    {}\n", line));
        }
        self.output.push_str("\n");
    }

    fn generate_list(&mut self, list: RstList) {
        match list {
            RstList::Unordered(items) => {
                for item in items {
                    self.generate_list_item(item, "- ");
                }
            }
            RstList::Ordered(items) => {
                let mut counter = 1;
                for item in items {
                    self.generate_list_item(item, &format!("{}. ", counter));
                    counter += 1;
                }
            }
        }
        self.output.push_str("\n");
    }

    fn generate_list_item(&mut self, item: RstListItem, prefix: &str) {
        self.output.push_str(prefix);
        for inline in item.content {
            self.generate_inline(inline);
        }
        self.output.push_str("\n");
    }
}

pub fn generate(root: RstRoot) -> (String, PanduckDiagnostics<String>) {
    let mut generator = GeneratorState::new();
    let output = generator.generate_root(root);
    (output, generator.diagnostics)
}
