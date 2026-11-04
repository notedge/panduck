use panduck_convert::{convert_bytes, MAX_STRUCTURE_DEPTH};

fn nested_html(depth: usize) -> String {
    let mut html = String::with_capacity(depth * 11 + 6);
    for _ in 0..depth {
        html.push_str("<div>");
    }
    html.push_str("body");
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html
}

fn nested_markdown(depth: usize) -> String {
    let mut markdown = String::from("# Deep\n\n");
    markdown.push_str(&"[".repeat(depth));
    markdown.push_str(&"]".repeat(depth));
    markdown.push('\n');
    markdown
}

fn assert_structure_budget_error(error: panduck_types::AdapterError) {
    let display = error.to_string();
    assert!(
        display.contains("structure nesting depth"),
        "the failure must name the structural guard: {display}"
    );
    match error {
        panduck_types::AdapterError::Diagnostic { diagnostic } => {
            assert_eq!(
                format!("{}", diagnostic.code()),
                "notedown.format.budget-exhausted",
                "the CLI maps this code to exit code 7"
            );
        }
        other => panic!("expected a structured diagnostic, got {other:?}"),
    }
}

#[test]
fn html_nesting_far_beyond_the_builtin_limit_returns_an_error_instead_of_aborting() {
    let html = nested_html(5000);
    let error = convert_bytes("html", "markdown", "deep.html", html.into_bytes(), None)
        .expect_err("5000 nested elements must be rejected before the parser runs");
    assert_structure_budget_error(error);
}

#[test]
fn markdown_bracket_nesting_far_beyond_the_builtin_limit_returns_an_error() {
    let markdown = nested_markdown(5000);
    let error = convert_bytes("markdown", "html", "deep.md", markdown.into_bytes(), None)
        .expect_err("5000 nested brackets must be rejected");
    assert_structure_budget_error(error);
}

#[test]
fn nesting_at_the_builtin_limit_still_converts() {
    let html = nested_html(MAX_STRUCTURE_DEPTH);
    let output = convert_bytes("html", "markdown", "limit.html", html.into_bytes(), None)
        .expect("depth at the built-in limit must stay convertible");
    assert!(output.markdown.contains("body"));
}
