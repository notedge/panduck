use panduck_core::{convert::parse_common_markdown, Result};
use panduck_latex::LaTeXConfig;

fn render_markdown(src: &str) -> Result<String> {
    let mut builder = LaTeXConfig::default().into_builder();
    let ast = parse_common_markdown(src)?;
    let out = builder.render_latex(&ast)?;
    return Ok(out);
}

#[test]
fn test() {
    let text = r#"
    # This is a test
    
    some text *here* and **here**
    "#;
    let out = render_markdown(text);
    println!("{:#?}", out);
}
