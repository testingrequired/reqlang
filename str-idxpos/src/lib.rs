use line_col::LineColLookup;

/// Map index to position (line, column)
///
/// Line and column are zero based
pub fn index_to_position(source: &str, index: usize) -> (usize, usize) {
    let lookup = LineColLookup::new(source);

    let (line, char) = lookup.get(index);

    (line - 1, char - 1)
}

/// Map position (line, column) to index
///
/// Line and column are zero based
pub fn position_to_index(source: &str, position: (usize, usize)) -> usize {
    let (line, character) = position;
    let lines = source.split('\n');
    let lines_before = lines.take(line);
    let line_chars_before = lines_before.fold(0usize, |acc, e| acc + e.len() + 1);
    let chars = character;

    line_chars_before + chars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_convert_index_to_position() {
        let source = "let a = 123;\nlet b = 456;";

        let index = 17usize;
        let expected_position = (1, 4);

        let index_to_position = index_to_position(source, index);
        let actual_position = index_to_position;

        assert_eq!(expected_position, actual_position);
    }

    #[test]
    fn it_should_convert_position_to_index() {
        let source = "let a = 123;\nlet b = 456;";
        let position = (1, 4);
        let expected_index = 17usize;
        let actual_index = position_to_index(source, position);

        assert_eq!(expected_index, actual_index);
    }

    #[test]
    fn it_should_convert_position_to_index_and_back() {
        let source = "let a = 123;\nlet b = 456;";
        let position = (1, 4);
        let actual_index = position_to_index(source, position);

        assert_eq!(position, index_to_position(source, actual_index));
    }

    #[test]
    fn it_should_convert_position_to_index_and_back_b() {
        let source = "let a = 123;\n{\n    let b = 456;\n}";
        let position = (2, 12);
        let actual_index = position_to_index(source, position);

        assert_eq!(position, index_to_position(source, actual_index));
    }

    #[test]
    fn it_should_convert_position_to_index_b() {
        let source = "let a = 123;\n{\n    let b = 456;\n}";
        let position = (2, 12);
        let actual_index = position_to_index(source, position);

        assert_eq!(27, actual_index);
    }

    #[test]
    fn it_should_convert_position_to_index_c() {
        let source = "let a = 123;\nlet b = 456;\nlet c = 789;";
        let position = (2, 8);
        let actual_index = position_to_index(source, position);

        assert_eq!(34, actual_index);
    }

    #[test]
    fn it_should_convert_position_to_index_d() {
        let source = "let a = 123;\nlet b = 456;\nlet c = 789;\nlet d = 000;";
        let position = (3, 8);
        let actual_index = position_to_index(source, position);

        assert_eq!(47, actual_index);
    }

    #[test]
    fn it_should_convert_position_to_index_e() {
        let source = "let a = 123;\nlet b = 456;\nlet c = 789;\nlet d = 000;\nlet e = 999;";
        let position = (4, 8);
        let actual_index = position_to_index(source, position);

        assert_eq!(60, actual_index);
    }

    #[test]
    fn it_should_convert_position_to_index_f() {
        let source = "let a = 123;\nlet b = 456;\nlet c = 789;\nlet d = 000;\nlet e = 999;\n";
        let position = (4, 8);
        let actual_index = position_to_index(source, position);

        assert_eq!(60, actual_index);
    }
}
