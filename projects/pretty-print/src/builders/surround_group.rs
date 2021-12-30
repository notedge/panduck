/// # SurroundGroup
///
/// ```toml
/// inline = "text"
/// multi = """
/// text
/// """
pub struct SurroundGroup {
    ident: usize,
    inline_start: String,
    inline_end: String,
    multiline_start: String,
    multiline_end: String,
}

impl Default for SurroundGroup {
    fn default() -> Self {
        Self {
            ident: 0,
            inline_start: "\"".to_string(),
            inline_end: "\"".to_string(),
            multiline_start: "\"\"\"".to_string(),
            multiline_end: "\"\"\"".to_string(),
        }
    }
}
