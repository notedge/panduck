use notedown_ast::ASTNode;

use panduck_core::Result;
use panduck_sycamore::SycamoreConfig;

mod docx;
mod html;
mod jupyter;
mod markdown;
mod rst;

#[test]
fn ready() {
    println!("it works!")
}

pub fn html_fragment_builder(parser: fn(&str) -> Result<ASTNode>) -> impl FnOnce(&str, &str) -> Result<()> {
    move |source, target| {
        let mut builder = SycamoreConfig::default().into_builder();
        let ast = parser(source)?;
        assert_eq!(builder.render(ast).to_string(), target);
        Ok(())
    }
}

pub fn html_standalone_builder(parser: fn(&str) -> Result<ASTNode>) -> impl FnOnce(&str, &str) -> Result<()> {
    move |source, target| {
        let mut builder = SycamoreConfig::default().into_builder();
        let ast = parser(source)?;
        assert_eq!(builder.render_standalone(ast).to_string(), target);
        Ok(())
    }
}
