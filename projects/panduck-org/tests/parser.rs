use crate::reader::parser::Parser;
use crate::ast::{OrgRoot, OrgNode, Heading, OrgBlock, OrgHeading, OrgInline};

#[test]
fn test_parse_empty_input() {
    let mut parser = Parser::new("");
    let org = parser.parse();
    assert!(org.blocks.is_empty());
}

#[test]
fn test_parse_heading() {
    let mut parser = Parser::new("* Title");
    let org = parser.parse();
    assert_eq!(org.blocks.len(), 1);
    match &org.blocks[0] {
        OrgBlock::Heading(heading) => {
            assert_eq!(heading.level, 1);
            assert_eq!(heading.content, vec![OrgInline::Text("Title".to_string())]);
        }
        _ => panic!("Expected a Heading node"),
    }
}