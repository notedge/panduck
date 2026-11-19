use document_formats::import::register_image_assets;
use document_ir::{AssetId, Block, DocumentGraph, DocumentId, Inline, ListItem, RelationEndpoint, RelationKind, TableCell, TableRow};

fn image(source: &str) -> Inline {
    Inline::Image { source: source.into(), alt: vec![Inline::Text { text: "diagram".into() }], title: None, asset: None }
}

fn bound_assets(block: &Block) -> Vec<Option<AssetId>> {
    let Block::Paragraph { content } = block
    else {
        return Vec::new();
    };
    content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Image { asset, .. } => Some(*asset),
            _ => None,
        })
        .collect()
}

#[test]
fn nested_blocks_register_each_image_occurrence_once() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    let paragraph = graph.push_block(Block::Paragraph { content: vec![image("diagram.png")] });
    let list = graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem { content: Vec::new(), children: vec![paragraph], number: None, ..Default::default() }],
    });
    graph.push_block(Block::Section { level: 1, title: Vec::new(), children: vec![list] });

    register_image_assets(&mut graph);

    assert_eq!(graph.assets.len(), 1);
    assert_eq!(graph.relations.len(), 1);
    assert_eq!(graph.relations[0].source, RelationEndpoint::Node(paragraph));
    assert_eq!(graph.relations[0].target, RelationEndpoint::Asset(graph.assets[0].id));
    assert_eq!(bound_assets(&graph.blocks[0].block), vec![Some(graph.assets[0].id)]);
    assert!(graph.validate().is_valid());
}

#[test]
fn table_images_keep_separate_occurrence_identity_and_node_endpoint() {
    let mut graph = DocumentGraph::new(DocumentId(2));
    let first = graph.push_block(Block::Paragraph { content: vec![image("same.png")] });
    let second = graph.push_block(Block::Paragraph { content: vec![image("same.png")] });
    graph.push_block(Block::Table {
        caption: None,
        head: Vec::new(),
        body: vec![TableRow {
            cells: vec![
                TableCell { children: vec![first], colspan: 1, rowspan: 1 },
                TableCell { children: vec![second], colspan: 1, rowspan: 1 },
            ],
        }],
        foot: Vec::new(),
    });

    register_image_assets(&mut graph);

    assert_eq!(graph.assets.len(), 2);
    assert_ne!(graph.assets[0].id, graph.assets[1].id);
    assert!(graph.assets.iter().all(|asset| asset.source.as_deref() == Some("same.png")));
    assert_eq!(graph.relations.len(), 2);
    assert!(graph.relations.iter().all(|relation| {
        relation.kind == RelationKind::Embeds && matches!(relation.source, RelationEndpoint::Node(node) if node == first || node == second)
    }));
    assert_eq!(bound_assets(&graph.blocks[0].block), vec![Some(graph.assets[0].id)]);
    assert_eq!(bound_assets(&graph.blocks[1].block), vec![Some(graph.assets[1].id)]);
    assert!(graph.validate().is_valid());
}
