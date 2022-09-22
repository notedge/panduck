use notedown_ast::nodes::CodeNode;
use notedown_error::Result;
use serde::{Deserialize, Serialize};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, Theme, ThemeSet},
    html::{append_highlighted_html_for_styled_line, highlighted_html_for_string, IncludeBackground},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyntectConfig {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Default for SyntectConfig {
    fn default() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            // base16-eighties.dark
            // base16-mocha.dark
            // base16-ocean.dark
            // base16-ocean.light
            // InspiredGitHub
            // Solarized (dark)
            // Solarized (light)
            theme: ThemeSet::load_defaults().themes["InspiredGitHub"].to_owned(),
        }
    }
}

impl SyntectConfig {
    pub fn render_html(&self, code: &CodeNode) -> Result<String> {
        match code.inline {
            true => self.render_inline(code),
            false => self.render_block(code),
        }
    }

    fn render_block(&self, code: &CodeNode) -> Result<String> {
        let ss = &self.syntax_set;
        let sr = ss.find_syntax_by_extension(&code.language).unwrap_or(ss.find_syntax_plain_text());
        let html = highlighted_html_for_string(&code.code, ss, sr, &self.theme).unwrap();
        return Ok(html);
    }
    fn render_inline(&self, code: &CodeNode) -> Result<String> {
        let ss = &self.syntax_set;
        let sr = ss.find_syntax_by_extension(&code.language).unwrap_or(ss.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(sr, &self.theme);
        //let (mut output, bg) = start_highlighted_html_snippet(theme);
        let mut output = String::from("<code>");
        let bg = self.theme.settings.background.unwrap_or(Color::WHITE);
        for line in LinesWithEndings::from(&code.code) {
            let regions = highlighter.highlight(line, ss);
            append_highlighted_html_for_styled_line(&regions[..], IncludeBackground::IfDifferent(bg), &mut output);
        }
        output.push_str("</code>");
        return Ok(output);
    }
}
