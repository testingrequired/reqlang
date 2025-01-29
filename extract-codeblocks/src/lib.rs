use markdown::{mdast::Node, to_mdast};
use span::Spanned;

/// Extract matching lang code blocks from a markdown string.
pub fn extract_codeblocks(input: impl AsRef<str>, lang: impl AsRef<str>) -> Vec<Spanned<String>> {
    let mut results = vec![];

    let md_options = markdown::ParseOptions::default();

    let nodes: Vec<Node> = to_mdast(input.as_ref(), &md_options)
        .unwrap()
        .children()
        .cloned()
        .unwrap_or_default();

    for node in &nodes {
        if let Node::Code(code) = node {
            let position = code.position.as_ref().unwrap();
            let start = position.start.offset;
            let end = position.end.offset;

            if let Some(codeblock_lang) = &code.lang {
                if codeblock_lang == lang.as_ref() {
                    let text = &code.value;
                    let span = start..end;

                    results.push((text.clone(), span));
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn extract_codeblock_matching_lang() {
        let input = textwrap::dedent(
            "
            ```javascript
            const foo = 123;
            ```

            ```test_lang
            TEST
            ```
            ",
        );

        assert_eq!(
            vec![(String::from("TEST"), 37..58)],
            extract_codeblocks(input, "test_lang")
        );
    }

    #[test]
    fn extract_multiple_codeblocks_matching_lang() {
        let input = textwrap::dedent(
            "
            ```javascript
            const foo = 123;
            ```

            ```test_lang
            TEST
            ```
            
            ```
            TEST
            ```

            ```test_lang
            TEST TEST
            ```
            ",
        );

        assert_eq!(
            vec![
                (String::from("TEST"), 37..58),
                (String::from("TEST TEST"), 74..100)
            ],
            extract_codeblocks(input, "test_lang")
        );
    }

    #[test]
    fn doesnt_exact_nonmatching_lang() {
        let input = textwrap::dedent(
            "
            ```javascript
            const foo = 123;
            ```

            ```test_lang
            TEST
            ```
            
            ```
            TEST
            ```

            ```test_lang
            TEST TEST
            ```
            ",
        );

        let expected: Vec<Spanned<String>> = vec![];

        assert_eq!(expected, extract_codeblocks(input, "nonmatching_lang"));
    }
}
