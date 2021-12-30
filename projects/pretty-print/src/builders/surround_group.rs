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

impl SurroundGroup {
    pub fn set_ident(&mut self, ident: usize) -> &mut Self {
        self.ident = ident;
        self
    }
    pub fn set_inline<S: Into<String>>(&mut self, start: S, end: S) -> &mut Self {
        self.inline_start = start.into();
        self.inline_end = end.into();
        self
    }
    pub fn set_multiline<S: Into<String>>(&mut self, start: S, end: S) -> &mut Self {
        self.multiline_start = start.into();
        self.multiline_end = end.into();
        self
    }
}
