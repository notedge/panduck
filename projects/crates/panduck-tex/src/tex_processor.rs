use mitex_parser::parse;
use mitex_spec::CommandSpec;

pub fn process_tex(input: &str) -> String {
    let spec = CommandSpec::default();
    let ast = parse(input, spec);
    // For now, just return a debug representation of the AST
    format!("Processed TeX AST: {:?}", ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_tex() {
        let input = "\\section{Hello World}";
        let output = process_tex(input);
        // Assert that the output contains some expected part of the AST representation
        assert!(output.contains("section"));
        assert!(output.contains("Hello World"));
    }
}