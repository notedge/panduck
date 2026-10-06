use std::fs;

use notedown_formats::export::markdown_project::{MarkdownProject, MarkdownProjectChapter};
use panduck_convert::publish_markdown_project;

#[test]
fn publish_markdown_project_writes_index_chapters_assets_and_report() {
    let project = MarkdownProject {
        index_markdown: "# Sample Book\n\n- [chapter one](chapters/000-chapter-one.md)\n".into(),
        chapters: vec![MarkdownProjectChapter {
            relative_path: "chapters/000-chapter-one.md".into(),
            markdown: "# Chapter One\n\nBody.\n".into(),
        }],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "assets/logo.png".into(),
            bytes: b"\x89PNG\r\n".to_vec(),
            asset_id: notedown_ir::AssetId(1),
        }],
        unresolved_asset_sources: Vec::new(),
    };
    let report = r#"{"schema_version":"panduck.report/v1","operation":"convert_project"}"#;
    let output_dir = std::env::temp_dir().join(format!("panduck-publish-project-{}", std::process::id()));

    let published = publish_markdown_project(&output_dir, &project, report).expect("publish project");

    assert_eq!(published.chapter_paths, vec!["chapters/000-chapter-one.md".to_string()]);
    assert_eq!(published.asset_paths, vec!["assets/logo.png".to_string()]);
    assert!(published.index_path.exists());
    assert!(output_dir.join("chapters/000-chapter-one.md").exists());
    assert!(output_dir.join("assets/logo.png").exists());
    assert!(published.report_path.exists());
    assert_eq!(
        fs::read_to_string(output_dir.join("chapters/000-chapter-one.md")).expect("chapter"),
        "# Chapter One\n\nBody.\n"
    );

    let _ = fs::remove_dir_all(&output_dir);
}
