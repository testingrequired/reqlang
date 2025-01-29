# extract-codeblocks

This crate extracts code blocks from Markdown text, returning the code and its span (start and end character offsets). It uses the `markdown` crate for parsing and the `span` crate (presumably a custom crate located in the `../span` directory) for handling spans.

## Usage

````rust
use extract_codeblocks::extract_code_blocks;

fn main() {
    let markdown = r#"
        ```rust
        println!("Hello, world!");
        ```
    "#;

    for (code, span) in extract_code_blocks(markdown, "rust") {
        println!("Code: {}", code); // println!("Hello, world!");
        println!("Span: {:?}", span); // 1..39
    }
}
````
