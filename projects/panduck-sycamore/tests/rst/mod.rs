use super::*;

pub fn html_standalone(source: &str, target: &str) -> Result<()> {
    html_standalone_builder(panduck_core::convert::parse_rst)(source, target)
}

#[test]
pub fn example() -> Result<()> {
    html_standalone(include_str!("example.rst"), include_str!("example.html"))?;
    Ok(())
}

#[test]
pub fn header() -> Result<()> {
    html_standalone(include_str!("header.rst"), include_str!("header.html"))?;
    Ok(())
}
