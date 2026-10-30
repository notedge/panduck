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
    let report = format!(r#"{{"schema_version":"{}","operation":"convert_project"}}"#, panduck_convert::SCHEMA_REPORT);
    let output_dir = std::env::temp_dir().join(format!("panduck-publish-project-{}", std::process::id()));

    let published = publish_markdown_project(&output_dir, &project, &report).expect("publish project");

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

/// Two consecutive publishes with identical input must produce identical
/// directory trees — same files, same content, same layout.
#[test]
fn repeated_publish_produces_deterministic_directory_tree() {
    let project = MarkdownProject {
        index_markdown: "# Deterministic\n\nHello.\n".into(),
        chapters: vec![
            MarkdownProjectChapter { relative_path: "chapters/000-a.md".into(), markdown: "# A\n\nBody A.\n".into() },
            MarkdownProjectChapter { relative_path: "chapters/001-b.md".into(), markdown: "# B\n\nBody B.\n".into() },
        ],
        assets: vec![
            notedown_formats::export::markdown_project::MarkdownProjectAsset {
                relative_path: "assets/a.png".into(),
                bytes: b"\x89PNG\x01".to_vec(),
                asset_id: notedown_ir::AssetId(1),
            },
            notedown_formats::export::markdown_project::MarkdownProjectAsset {
                relative_path: "assets/b.png".into(),
                bytes: b"\x89PNG\x02".to_vec(),
                asset_id: notedown_ir::AssetId(2),
            },
        ],
        unresolved_asset_sources: Vec::new(),
    };
    let report = format!(r#"{{"schema_version":"{}"}}"#, panduck_convert::SCHEMA_REPORT);
    let base = std::env::temp_dir().join(format!("panduck-deterministic-{}", std::process::id()));
    let dir_a = base.join("a");
    let dir_b = base.join("b");

    publish_markdown_project(&dir_a, &project, &report).expect("first publish");
    publish_markdown_project(&dir_b, &project, &report).expect("second publish");

    let a_entries = sorted_entries(&dir_a);
    let b_entries = sorted_entries(&dir_b);
    assert_eq!(a_entries, b_entries, "two publishes should yield identical file trees");

    let _ = fs::remove_dir_all(&base);
}

/// A publish that fails mid-flow (e.g. bad asset path) must not mutate a
/// previously successful output directory — the directory must remain a
/// coherent snapshot of the last successful publish.
#[test]
fn failed_publish_does_not_pollute_existing_output() {
    let good_project = MarkdownProject {
        index_markdown: "# Good\n".into(),
        chapters: vec![],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "assets/ok.png".into(),
            bytes: b"ok".to_vec(),
            asset_id: notedown_ir::AssetId(1),
        }],
        unresolved_asset_sources: Vec::new(),
    };
    // Bad asset path: does not start with `assets/` → fails inside the loop.
    let bad_project = MarkdownProject {
        index_markdown: "# Bad\n".into(),
        chapters: vec![MarkdownProjectChapter { relative_path: "chapters/000.md".into(), markdown: "# Mid\n".into() }],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "not-in-assets-dir.png".into(),
            bytes: b"nope".to_vec(),
            asset_id: notedown_ir::AssetId(2),
        }],
        unresolved_asset_sources: Vec::new(),
    };
    let report = format!(r#"{{"schema_version":"{}"}}"#, panduck_convert::SCHEMA_REPORT);
    let output_dir = std::env::temp_dir().join(format!("panduck-failure-isolated-{}", std::process::id()));

    publish_markdown_project(&output_dir, &good_project, &report).expect("initial publish");
    let before = sorted_entries(&output_dir);

    let result = publish_markdown_project(&output_dir, &bad_project, &report);
    assert!(result.is_err(), "bad asset path must produce an error");

    let after = sorted_entries(&output_dir);
    assert_eq!(before, after, "failed publish must not mutate existing output");

    let _ = fs::remove_dir_all(&output_dir);
}

/// Publishing into a directory whose parents do not exist must create the
/// whole chain, not fail on a missing parent.
#[test]
fn publish_markdown_project_creates_missing_nested_output_directory() {
    let base = unique_temp_dir("panduck-nested");
    let output_dir = base.join("nested").join("site");
    let project = MarkdownProject {
        index_markdown: "# Nested\n".into(),
        chapters: vec![],
        assets: vec![],
        unresolved_asset_sources: Vec::new(),
    };
    let report = format!(r#"{{"schema_version":"{}"}}"#, panduck_convert::SCHEMA_REPORT);

    let published = publish_markdown_project(&output_dir, &project, &report).expect("publish nested");

    assert!(output_dir.is_dir());
    assert_eq!(fs::read_to_string(&published.index_path).expect("index"), "# Nested\n");
    let _ = fs::remove_dir_all(&base);
}

/// Republishing owns the output directory: files from the previous layout that
/// the new project does not declare must disappear rather than linger.
#[test]
fn repeated_publish_replaces_stale_output_contents() {
    let base = unique_temp_dir("panduck-republish");
    let output_dir = base.join("site");
    let report = format!(r#"{{"schema_version":"{}"}}"#, panduck_convert::SCHEMA_REPORT);
    let first = MarkdownProject {
        index_markdown: "# First\n".into(),
        chapters: vec![MarkdownProjectChapter { relative_path: "chapters/000-a.md".into(), markdown: "# A\n".into() }],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "assets/old.png".into(),
            bytes: b"old".to_vec(),
            asset_id: notedown_ir::AssetId(1),
        }],
        unresolved_asset_sources: Vec::new(),
    };
    let second = MarkdownProject {
        index_markdown: "# Second\n".into(),
        chapters: vec![],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "assets/new.png".into(),
            bytes: b"new".to_vec(),
            asset_id: notedown_ir::AssetId(2),
        }],
        unresolved_asset_sources: Vec::new(),
    };

    publish_markdown_project(&output_dir, &first, &report).expect("first publish");
    fs::write(output_dir.join("stale.md"), "stale").expect("write stale file");

    let published = publish_markdown_project(&output_dir, &second, &report).expect("second publish");

    assert_eq!(fs::read_to_string(&published.index_path).expect("index"), "# Second\n");
    assert!(output_dir.join("assets/new.png").exists());
    assert!(!output_dir.join("assets/old.png").exists(), "old assets must not survive a republish");
    assert!(!output_dir.join("stale.md").exists(), "unowned files must not survive a republish");
    assert!(!output_dir.join("chapters").exists(), "old chapter directories must not survive a republish");
    assert_eq!(fs::read_dir(&base).expect("base entries").count(), 1, "only the output directory may remain");
    let _ = fs::remove_dir_all(&base);
}

/// A failed first publish must create neither the target directory nor a
/// leftover staging sibling.
#[test]
fn failed_first_publish_leaves_no_output_or_staging_directory() {
    let base = unique_temp_dir("panduck-failure-clean");
    fs::create_dir_all(&base).expect("base");
    let output_dir = base.join("site");
    let bad_project = MarkdownProject {
        index_markdown: "# Bad\n".into(),
        chapters: vec![MarkdownProjectChapter { relative_path: "chapters/000.md".into(), markdown: "# Mid\n".into() }],
        assets: vec![notedown_formats::export::markdown_project::MarkdownProjectAsset {
            relative_path: "outside-assets.png".into(),
            bytes: b"nope".to_vec(),
            asset_id: notedown_ir::AssetId(1),
        }],
        unresolved_asset_sources: Vec::new(),
    };
    let report = format!(r#"{{"schema_version":"{}"}}"#, panduck_convert::SCHEMA_REPORT);

    let result = publish_markdown_project(&output_dir, &bad_project, &report);

    assert!(result.is_err(), "bad asset path must produce an error");
    assert!(!output_dir.exists(), "failed publish must not create the target directory");
    let leftovers: Vec<_> = fs::read_dir(&base)
        .expect("base entries")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().to_string())
        .collect();
    assert!(leftovers.is_empty(), "failed publish leaked staging entries: {leftovers:?}");
    let _ = fs::remove_dir_all(&base);
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn sorted_entries(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    walk(dir, dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        let rel = path.strip_prefix(root).expect("strip");
        if path.is_dir() {
            walk(root, &path, out);
        }
        else {
            let bytes = fs::read(&path).expect("read file");
            out.push((rel.to_string_lossy().to_string(), bytes));
        }
    }
}
