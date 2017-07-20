use panduck_markdown::{parse_and_generate_markdown, MarkdownReadConfig};

#[test]
fn test_integration_heading() {
    let markdown = "# Hello, World!";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<h1>Hello, World!</h1>\n");
}

#[test]
fn test_integration_paragraph() {
    let markdown = "This is a paragraph.";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<p>This is a paragraph.</p>\n");
}

#[test]
fn test_integration_bold_italic() {
    let markdown = "**bold** _italic_";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<p><strong>bold</strong> <em>italic</em></p>\n");
}

#[test]
fn test_integration_code_block() {
    let markdown = "```rust\nfn main() {}\n```";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<pre><code class=\"language-rust\">fn main() {}\n</code></pre>\n");
}

#[test]
fn test_integration_list() {
    let markdown = "- Item 1\n- Item 2";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<ul>\n<li>Item 1</li>\n<li>Item 2</li>\n</ul>\n");
}

#[test]
fn test_integration_link_image() {
    let markdown = "[Google](https://www.google.com) ![Alt Text](image.png)";
    let config = MarkdownReadConfig { support_math: false };
    let result = parse_and_generate_markdown(markdown, config);
    assert!(!result.has_errors());
    assert_eq!(result.into_value(), "<p><a href=\"https://www.google.com\">Google</a> <img src=\"image.png\" alt=\"Alt Text\"></p>\n");
}