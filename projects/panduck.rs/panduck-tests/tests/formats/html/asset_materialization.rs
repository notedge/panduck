//! HTML image asset materialization contract.

use std::collections::HashMap;

use document_formats::import::html::{HtmlAssetSource, import_html_text, import_html_with_links, import_html_with_links_and_assets};
use document_ir::{Inline, SemanticStatus};

struct Bytes(HashMap<String, Vec<u8>>);

impl HtmlAssetSource for Bytes {
    fn read_asset(&self, target: &str) -> Option<Vec<u8>> {
        self.0.get(target).cloned()
    }
}

fn png_bytes() -> Vec<u8> {
    vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}

#[test]
fn captured_image_bytes_resolve_the_asset_with_a_content_identity() {
    let mut bytes = Bytes(HashMap::new());
    bytes.0.insert("assets/a.png".into(), png_bytes());
    let graph = import_html_with_links_and_assets("assets.html", r#"<body><p><img src="assets/a.png" alt="A"></p></body>"#, &bytes)
        .expect("import with assets");

    let asset = graph.document.assets.first().expect("image asset");
    assert_eq!(asset.status, SemanticStatus::Resolved);
    assert_eq!(asset.bytes.as_deref(), Some(png_bytes().as_slice()));
    assert_eq!(asset.media_type.as_deref(), Some("image/png"));
    let identity = asset.content_identity.as_deref().expect("content identity");
    assert_eq!(identity, blake3::hash(&png_bytes()).to_hex().to_string());
    assert!(!graph.document.coverage.loss.iter().any(|loss| loss.code == "import.html.asset_unresolved"));
}

#[test]
fn missing_image_bytes_keep_the_asset_unresolved_and_raise_a_loss() {
    let graph = import_html_text("assets.html", r#"<body><p><img src="assets/a.png" alt="A"></p></body>"#).expect("import without assets");
    let asset = graph.assets.first().expect("image asset");
    assert_eq!(asset.status, SemanticStatus::Unresolved);
    assert!(asset.bytes.is_none());
    assert!(asset.content_identity.is_none());
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.html.asset_unresolved" && loss.message.contains("assets/a.png")));
}

#[test]
fn image_bytes_infer_the_media_type_when_the_target_has_no_extension() {
    let mut bytes = Bytes(HashMap::new());
    bytes.0.insert("blob".into(), png_bytes());
    let graph = import_html_with_links_and_assets("assets.html", r#"<body><img src="blob"></body>"#, &bytes).expect("import");
    assert_eq!(graph.document.assets[0].media_type.as_deref(), Some("image/png"));
}

#[test]
fn srcset_without_src_selects_the_largest_candidate() {
    let import = import_html_with_links("srcset.html", r#"<body><img srcset="small.png 1x, large.png 3x, medium.png 2x" alt="S"></body>"#)
        .expect("import");
    let image_source = image_source(&import.document);
    assert_eq!(image_source, "large.png");
    assert_eq!(import.links.iter().filter(|link| link.role == document_formats::import::html::HtmlLinkRole::SrcSet).count(), 3);
}

#[test]
fn srcset_width_descriptors_select_the_widest_candidate() {
    let import = import_html_with_links("srcset.html", r#"<body><img srcset="a.png 320w, b.png 1280w, c.png 640w"></body>"#).expect("import");
    assert_eq!(image_source(&import.document), "b.png");
}

#[test]
fn authored_src_wins_over_srcset_candidates() {
    let import = import_html_with_links("srcset.html", r#"<body><img src="chosen.png" srcset="large.png 3x"></body>"#).expect("import");
    assert_eq!(image_source(&import.document), "chosen.png");
}

fn image_source(graph: &document_ir::DocumentGraph) -> String {
    for node in &graph.blocks {
        if let Some(source) = find_image(&node.block) {
            return source;
        }
    }
    panic!("no image block: {:?}", graph.blocks);
}

fn find_image(block: &document_ir::Block) -> Option<String> {
    let content = match block {
        document_ir::Block::Paragraph { content } => content.as_slice(),
        _ => return None,
    };
    content.iter().find_map(|inline| match inline {
        Inline::Image { source, .. } => Some(source.clone()),
        Inline::Styled { children, .. } | Inline::Attributed { children, .. } => children.iter().find_map(|child| match child {
            Inline::Image { source, .. } => Some(source.clone()),
            _ => None,
        }),
        _ => None,
    })
}
