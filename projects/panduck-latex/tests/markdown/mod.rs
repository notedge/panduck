use panduck_core::{convert::parse_common_markdown, Result};
use panduck_latex::LaTeXConfig;

pub fn check_markdown(source: &str, target: &str) -> Result<()> {
    let mut builder = LaTeXConfig::default().into_builder();
    let ast = parse_common_markdown(source)?;
    let out = builder.render_latex(&ast)?;
    Ok(assert_eq!(out, target))
}

#[test]
fn basic() -> Result<()> {
    check_markdown(include_str!("basic.md"), include_str!("basic.tex"))
}
