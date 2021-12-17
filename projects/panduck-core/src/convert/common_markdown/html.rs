use super::*;
use comrak::nodes::NodeHtmlBlock;

impl ToNotedown for NodeHtmlBlock {
    fn into_notedown(self) -> ASTNode {
        let html = String::from_utf8_lossy(&self.literal);
        ASTKind::code_block(html, "html", None)
    }
}
