use super::*;

/// dynamic info
pub struct LaTeXContext {
    pub vm: NoteVM,
    pub is_orderless_env: bool,
}

impl Default for LaTeXContext {
    fn default() -> Self {
        Self { vm: Default::default(), is_orderless_env: false }
    }
}
