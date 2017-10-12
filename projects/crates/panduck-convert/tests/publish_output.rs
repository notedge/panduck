use panduck_convert::{publish_bytes, publish_text};
use std::fs;

#[test]
fn publish_text_replaces_existing_output() {
    let base = std::env::temp_dir().join(format!("panduck-publish-text-{}", std::process::id()));
    fs::create_dir_all(&base).expect("create temp dir");
    let target = base.join("out.md");

    publish_text(&target, "v1").expect("first publish");
    publish_text(&target, "v2").expect("replace publish");
    assert_eq!(fs::read_to_string(&target).expect("read"), "v2");

    let _ = fs::remove_dir_all(base);
}

#[test]
fn publish_bytes_writes_binary_output() {
    let base = std::env::temp_dir().join(format!("panduck-publish-binary-{}", std::process::id()));
    fs::create_dir_all(&base).expect("create temp dir");
    let target = base.join("out.bin");

    publish_bytes(&target, &[0x50, 0x4b, 0x03, 0x04]).expect("publish binary");
    assert_eq!(fs::read(&target).expect("read"), [0x50, 0x4b, 0x03, 0x04]);

    let _ = fs::remove_dir_all(base);
}
