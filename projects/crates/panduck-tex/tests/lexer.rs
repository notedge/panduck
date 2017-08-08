use panduck_tex::{parse_and_generate_markdown, MarkdownReadConfig};
use panduck_types::AdapterError;

#[test]
fn tex_lexer_path_is_not_wired() {
    let err = parse_and_generate_markdown("text", MarkdownReadConfig { support_math: false })
        .unwrap_err();
    assert!(matches!(err, AdapterError::NotImplemented { .. }));
}
