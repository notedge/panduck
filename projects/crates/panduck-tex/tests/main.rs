use panduck_tex::{parse_and_generate_markdown, MarkdownReadConfig};
use panduck_types::AdapterError;

#[test]
fn tex_pipeline_reports_not_implemented() {
    let err = parse_and_generate_markdown("# heading", MarkdownReadConfig { support_math: false })
        .unwrap_err();
    assert!(matches!(
        err,
        AdapterError::NotImplemented { feature } if feature == "tex markdown pipeline"
    ));
}
