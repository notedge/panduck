## Markdown Lexer

This module provides a lexer for Markdown, converting a raw Markdown string into a stream of `MarkdownTokenType` tokens.

It reuses the generic `LexerState` from `panduck-types` to handle common lexing concerns such as character iteration, position tracking, and token collection.

### Usage

```rust
use panduck_types::helpers::SourceText;
use panduck_markdown::lexer::MarkdownLexer;

let source_text = SourceText::new("# Hello World");
let lexer = MarkdownLexer::new();
let diagnostics = lexer.lex(&source_text);

match diagnostics.result {
    Ok(token_stream) => {
        for token in token_stream.tokens() {
            println!("{:?}", token);
        }
    }
    Err(e) => {
        eprintln!("Lexing error: {:?}", e);
    }
}
```