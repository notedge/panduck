use panduck_core::{convert::parse_html, Result};
use panduck_html::HTMLConfig;

#[test]
fn nested() -> Result<()> {
    let mut cfg = HTMLConfig::default().into_builder();
    let ast = parse_html("<p>text<i>i</i><b>b</b></p>")?;
    let out = cfg.render(&ast)?;
    println!("{}", out);
    Ok(())
}
