use document_formats::export::markdown_project::export_markdown_project;
use document_ir::{
    Asset, AssetId, AssetKind, Block, DocumentGraph, DocumentId, Inline, ListItem, NodeId, SemanticStatus, SourceKind, SourceRef,
};

#[test]
fn export_markdown_project_materializes_assets_and_rewrites_image_paths() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("media/logo.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(b"\x89PNG\r\n".to_vec()),
    });
    graph.push_block_with_id(
        NodeId(1),
        Block::Paragraph {
            content: vec![Inline::Image {
                source: "media/logo.png".into(),
                alt: vec![Inline::Text { text: "Logo".into() }],
                title: None,
                asset: Some(AssetId(1)),
            }],
        },
    );

    let project = export_markdown_project(&graph).expect("export project");
    assert!(project.index_markdown.contains("![Logo](assets/logo.png)"));
    assert_eq!(project.assets.len(), 1);
    assert_eq!(project.assets[0].relative_path, "assets/logo.png");
    assert_eq!(project.assets[0].bytes, b"\x89PNG\r\n");
    assert!(project.unresolved_asset_sources.is_empty());
    assert!(project.chapters.is_empty());
}

#[test]
fn export_markdown_project_derives_asset_extension_from_media_type() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("pdf-object-7".into()),
        media_type: Some("image/jpeg".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(b"\xFF\xD8\xFF\xD9".to_vec()),
    });
    graph.push_block_with_id(
        NodeId(1),
        Block::Paragraph {
            content: vec![Inline::Image { source: "pdf-object-7".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
        },
    );

    let project = export_markdown_project(&graph).expect("export project");
    assert_eq!(project.assets[0].relative_path, "assets/pdf-object-7.jpg");
    assert!(project.index_markdown.contains("![](assets/pdf-object-7.jpg)"));
}

#[test]
fn export_markdown_project_deduplicates_assets_by_content_fingerprint() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    for (id, source) in [(1, "media/a.png"), (2, "media/b.png")] {
        graph.push_asset(Asset {
            id: AssetId(id),
            kind: AssetKind::Image,
            content_identity: Some("blake3:identical".into()),
            source: Some(source.into()),
            media_type: Some("image/png".into()),
            status: SemanticStatus::Resolved,
            bytes: Some(b"same-bytes".to_vec()),
        });
    }
    for (node_id, asset_id, source) in [(1, 1, "media/a.png"), (2, 2, "media/b.png")] {
        graph.push_block_with_id(
            NodeId(node_id),
            Block::Paragraph {
                content: vec![Inline::Image {
                    source: source.into(),
                    alt: vec![Inline::Text { text: format!("Image {asset_id}") }],
                    title: None,
                    asset: Some(AssetId(asset_id)),
                }],
            },
        );
    }

    let project = export_markdown_project(&graph).expect("export project");
    assert_eq!(project.assets.len(), 1, "identical content must materialize once");
    assert_eq!(project.assets[0].relative_path, "assets/a.png");
    assert!(project.index_markdown.contains("![Image 1](assets/a.png)"));
    assert!(project.index_markdown.contains("![Image 2](assets/a.png)"));
}

#[test]
fn export_markdown_project_keeps_unresolved_sources_in_markdown() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("media/missing.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Unresolved,
        bytes: None,
    });
    graph.push_block_with_id(
        NodeId(1),
        Block::Paragraph {
            content: vec![Inline::Image {
                source: "media/missing.png".into(),
                alt: vec![Inline::Text { text: "Missing".into() }],
                title: None,
                asset: Some(AssetId(1)),
            }],
        },
    );

    let project = export_markdown_project(&graph).expect("export project");
    assert!(project.index_markdown.contains("![Missing](media/missing.png)"));
    assert!(project.assets.is_empty());
    assert_eq!(project.unresolved_asset_sources, vec!["media/missing.png".to_string()]);
}

#[test]
fn export_markdown_project_splits_epub_spine_members_into_chapters() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.metadata.title = Some("Sample Book".into());
    let first = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Chapter One".into() }], children: vec![] });
    graph.attach_source(SourceRef {
        node: first,
        kind: SourceKind::PackageMember { path: "OEBPS/chapter-one.xhtml".into() },
        precision: SemanticStatus::Partial,
    });
    let second = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Chapter Two".into() }], children: vec![] });
    graph.attach_source(SourceRef {
        node: second,
        kind: SourceKind::PackageMember { path: "OEBPS/chapter-two.xhtml".into() },
        precision: SemanticStatus::Partial,
    });

    let project = export_markdown_project(&graph).expect("export project");
    assert_eq!(project.chapters.len(), 2);
    assert!(project.index_markdown.contains("# Sample Book"));
    assert!(project.index_markdown.contains("chapters/000-chapter-one.md"));
    assert!(project.index_markdown.contains("chapters/001-chapter-two.md"));
    assert!(project.chapters[0].markdown.contains("# Chapter One"));
    assert!(project.chapters[1].markdown.contains("# Chapter Two"));
}

#[test]
fn export_markdown_project_reports_omitted_navigation_member() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.metadata.title = Some("Nav Book".into());
    let nav = graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem {
            content: vec![Inline::Text { text: "Part A".into() }],
            children: Vec::new(),
            number: None,
            is_task: false,
            is_checked: None,
        }],
    });
    graph.attach_source(SourceRef {
        node: nav,
        kind: SourceKind::PackageMember { path: "OEBPS/nav.xhtml".into() },
        precision: SemanticStatus::Partial,
    });
    let chapter = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Chapter One".into() }], children: vec![] });
    graph.attach_source(SourceRef {
        node: chapter,
        kind: SourceKind::PackageMember { path: "OEBPS/chapter.xhtml".into() },
        precision: SemanticStatus::Partial,
    });

    let project = export_markdown_project(&graph).expect("export project");
    assert_eq!(project.chapters.len(), 1);
    let loss = project.losses.iter().find(|loss| loss.code == "export.markdown_project.navigation_omitted").expect("navigation loss");
    assert_eq!(loss.status, SemanticStatus::Lossy);
    assert!(loss.message.contains("OEBPS/nav.xhtml"), "{}", loss.message);
    assert!(loss.message.contains("#part-a"), "{}", loss.message);
}
