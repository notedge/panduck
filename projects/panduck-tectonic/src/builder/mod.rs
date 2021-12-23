use tectonic;

pub struct TectonicBuilder {
    config: TectonicConfig,
    context: TectonicContext,
}
// static info
pub struct TectonicConfig {}
/// dynamic info
pub struct TectonicContext {}

pub trait IntoTectonic {
    fn into_sycamore(self, cfg: &TectonicConfig, ctx: &mut TectonicContext);
}

#[test]
fn test() {
    let latex = r#"
\documentclass{article}
\begin{document}
Hello, world!
\end{document}
"#;

    let pdf_data: Vec<u8> = tectonic::latex_to_pdf(latex).expect("processing failed");
    println!("Output PDF size is {} bytes", pdf_data.len());
}
