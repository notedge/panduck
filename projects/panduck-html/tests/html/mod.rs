use panduck_core::{convert::parse_html, Result};
use panduck_html::PrettyRenderer;

#[test]
fn nested() -> Result<()> {
    let mut cfg = PrettyRenderer::default();
    let ast = parse_html("<p>text<i>i</i><b>b</b></p>")?;
    let out = cfg.render(&ast)?;
    println!("{}", out);
    Ok(())
}
