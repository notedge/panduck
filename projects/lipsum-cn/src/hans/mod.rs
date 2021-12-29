use handlebars::Handlebars;

fn main() -> Result<(), Box<dyn Error>> {
    let mut reg = Handlebars::new();
    // render without register
    println!("{}", reg.render_template("Hello {{name}}", &json!({"name": "foo"}))?);

    // register template using given name
    reg.register_template_string("tpl_1", "Good afternoon, {{name}}")?;
    println!("{}", reg.render("tpl_1", &json!({"name": "foo"}))?);

    Ok(())
}

pub static 名言模板: &'static =

pub fn 生成名言() {
    include_str!("名言.txt")
}