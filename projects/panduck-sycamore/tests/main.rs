use panduck_core::{convert, Result};
use panduck_sycamore::SycamoreBuilder;
mod markdown;

#[test]
fn ready() {
    println!("it works!")
}

pub fn html_fragment(source: &str, target: &str) -> Result<()> {
    let config = SycamoreBuilder::default();
    let ast = convert::parse_common_markdown(source)?;
    assert_eq!(config.render(ast).to_string(), target);
    Ok(())
}

pub fn html_standalone(source: &str, target: &str) -> Result<()> {
    let config = SycamoreBuilder::default();
    let ast = convert::parse_common_markdown(source)?;
    assert_eq!(config.render_standalone(ast).to_string(), target);
    Ok(())
}
