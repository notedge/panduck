use crate::writer::generator::generate;
use crate::ast::{OrgRoot, OrgBlock, OrgHeading, OrgInline};

#[test]
fn test_generate_empty_org() {
    let org = OrgRoot { blocks: Vec::new() };
    let expected = "";
    assert_eq!(generate(&org), expected);
}

#[test]
fn test_generate_heading() {
    let mut org = OrgRoot { blocks: Vec::new() };
    let heading = OrgHeading {
        level: 1,
        content: vec![OrgInline::Text("Title".to_string())],
    };
    org.blocks.push(OrgBlock::Heading(heading));
    let expected = "* Title\n";
    assert_eq!(generate(&org), expected);
}