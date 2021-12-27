use comrak::nodes::{ListType, NodeDescriptionItem, NodeList};
use notedown_ast::nodes::{ListItem, ListPrefixSymbol, ListView, Literal, QuoteBlock};

use super::*;

impl ToNotedown for NodeDescriptionItem {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }
}

fn node_prefix(kind: &NodeList) -> ListPrefixSymbol {
    match kind.list_type {
        ListType::Bullet => ListPrefixSymbol::Hyphen,
        ListType::Ordered => ListPrefixSymbol::ArabicNest { prefix_number: vec![], number: kind.start },
    }
}

pub fn node_list<'a>(_: NodeList, nodes: ASTNodes) -> ASTNode {
    match &nodes.len() {
        0 => unimplemented!("{:#?}", nodes),
        1 => nodes.get(0).unwrap().to_owned(),
        _ => {
            let mut nodes = nodes.iter();
            let mut view = match nodes.next().and_then(|f| f.as_list_view()) {
                Some(s) => s,
                None => return ASTNode::default(),
            };
            let children = view.children_mut();
            for i in nodes {
                if let Some(s) = i.as_list_view() {
                    children.extend_from_slice(&s.children())
                }
            }
            return view.into();
        }
    }
}

pub fn node_item<'a>(kind: NodeList, nodes: ASTNodes) -> ASTNode {
    let prefix = node_prefix(&kind);
    let item = ListItem { prefix: Literal { value: prefix.clone(), range: None }, rest: nodes };
    match &prefix {
        x if x.is_quote() => QuoteBlock::quote(item.rest).into(),
        x if x.is_ordered() => ListView::ordered_list(vec![item]).into(),
        _ => ListView::orderless_list(vec![item]).into(),
    }
}

pub fn block_quote<'a>(node: &'a AstNode<'a>) -> ASTNode {
    let nodes = node.children().into_notedown_list();
    match &nodes.len() {
        0 => unimplemented!("{:#?}", nodes),
        1 => {
            let node = nodes.get(0);
            unimplemented!("{:#?}", node)
        }
        _ => {
            let item = ListItem {
                prefix: Literal { value: ListPrefixSymbol::Quote, range: None },
                rest: node.children().into_notedown_list(),
            };
            unimplemented!("{:#?}", item)
        }
    }
}
