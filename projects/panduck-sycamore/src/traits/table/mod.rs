use super::*;

impl<G> IntoSycamore<G> for TableView
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        todo!()
    }
}
