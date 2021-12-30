/// # Prefix Group
///
/// ## Examples
///
/// ### Markdown Quote
///
/// ```
/// let mut group = panduck_pp::PrefixGroup::default();
/// group.set_mark("> ", "> ");
/// ```
///
/// ```markdown
/// > text1 text2 text3
///
/// > text1
/// > text2
/// > text3
/// ```
///
/// ### Markdown Orderless List
///
/// ```
/// let mut group = panduck_pp::PrefixGroup::default();
/// group.set_mark("> ", "> ");
/// ```
///
/// ```markdown
/// - text1 text2 text3
///
/// - text1
///   text2
///   text3
/// ```
pub struct PrefixGroup {
    head_mark: String,
    rest_mark: String,
}

impl Default for PrefixGroup {
    fn default() -> Self {
        Self { head_mark: "> ".to_string(), rest_mark: "> ".to_string() }
    }
}

impl PrefixGroup {
    pub fn set_mark<S: Into<String>>(&mut self, first: S, rest: S) -> &mut Self {
        self.head_mark = first.into();
        self.rest_mark = rest.into();
        self
    }
}

impl PrefixGroup {
    #[inline]
    pub fn pretty_print() {}
    #[inline]
    pub fn pretty_render() {}

    pub fn build() {}
}
