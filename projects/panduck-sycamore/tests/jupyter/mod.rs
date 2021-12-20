use super::*;

pub fn html_standalone(source: &str, target: &str) -> Result<()> {
    html_standalone_builder(panduck_core::convert::parse_jupyter)(source, target)
}

#[test]
fn md_full() -> Result<()> {
    html_standalone(include_str!("ipython.json"), include_str!("ipython.html"))?;
    Ok(())
}
