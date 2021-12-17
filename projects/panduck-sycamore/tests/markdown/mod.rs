use super::*;

#[test]
pub fn basic() -> Result<()> {
    html_standalone(include_str!("basic.md"), include_str!("basic.html"))?;
    Ok(())
}

#[test]
pub fn extensions() -> Result<()> {
    html_standalone(include_str!("extensions.md"), include_str!("extensions.html"))?;
    Ok(())
}

#[test]
pub fn regression() -> Result<()> {
    html_standalone(include_str!("regression.md"), include_str!("regression.html"))?;
    Ok(())
}

#[test]
pub fn markdown_it() -> Result<()> {
    html_standalone(include_str!("markdown-it.md"), include_str!("markdown-it.html"))?;
    Ok(())
}

#[test]
pub fn punctuation() -> Result<()> {
    html_standalone(include_str!("punctuation.md"), include_str!("punctuation.html"))?;
    Ok(())
}
