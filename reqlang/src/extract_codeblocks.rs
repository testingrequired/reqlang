use markdown::{mdast::Node, to_mdast};

use crate::span::Spanned;

pub type SpannedContent = Spanned<String>;
pub type SpannedCodeBlock = Spanned<SpannedContent>;

/// Extract matching lang code blocks from a markdown string.
pub fn extract_codeblocks(
    input: impl AsRef<str>,
    target_lang: impl AsRef<str>,
) -> Vec<SpannedCodeBlock> {
    let mut results = vec![];

    let md_nodes: Vec<Node> = to_mdast(input.as_ref(), &markdown::ParseOptions::default())
        .unwrap()
        .children()
        .cloned()
        .unwrap_or_default();

    for md_node in md_nodes {
        if let Node::Code(codeblock) = &md_node {
            let position = codeblock.position.as_ref().unwrap();
            let start = position.start.offset;
            let end = position.end.offset;

            let text = codeblock.value.clone();
            let text_start = (3 + 1) + target_lang.as_ref().len() + start;
            let text_end = text_start + text.len();

            if let Some(lang) = &codeblock.lang {
                if lang == target_lang.as_ref() {
                    results.push(((text, text_start..text_end), start..end));
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
            vec![((String::from("TEST"), 50..54), 37..58)],
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
                ((String::from("TEST"), 50..54), 37..58),
                ((String::from("TEST TEST"), 87..96), 74..100)
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

        let expected: Vec<SpannedCodeBlock> = vec![];

        assert_eq!(expected, extract_codeblocks(input, "nonmatching_lang"));
    }
}
