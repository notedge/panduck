use notedown_ast::ASTNode;
use panduck_core::Result;
use panduck_sycamore::SycamoreBuilder;

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
        let config = SycamoreBuilder::default();
        let ast = parser(source)?;
        assert_eq!(config.render(ast).to_string(), target);
        Ok(())
    }
}

pub fn html_standalone_builder(parser: fn(&str) -> Result<ASTNode>) -> impl FnOnce(&str, &str) -> Result<()> {
    move |source, target| {
        let config = SycamoreBuilder::default();
        let ast = parser(source)?;
        assert_eq!(config.render_standalone(ast).to_string(), target);
        Ok(())
    }
}
