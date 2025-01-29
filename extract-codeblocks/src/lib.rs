use markdown::{mdast::Node, to_mdast};
use span::Spanned;

/// Extract matching lang code blocks from a markdown string.
pub fn extract_codeblocks(input: impl AsRef<str>, lang: impl AsRef<str>) -> Vec<Spanned<String>> {
    let mut results = vec![];

    let md_nodes: Vec<Node> = to_mdast(input.as_ref(), &markdown::ParseOptions::default())
        .unwrap()
        .children()
        .cloned()
        .unwrap_or_default();

    for md_node in md_nodes {
        if let Node::Code(codeblock) = md_node {
            let position = codeblock.position.as_ref().unwrap();
            let start = position.start.offset;
            let end = position.end.offset;

            if let Some(codeblock_lang) = &codeblock.lang {
                if codeblock_lang == lang.as_ref() {
                    let codeblock_text = codeblock.value;
                    let codeblock_span = start..end;

                    results.push((codeblock_text, codeblock_span));
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
