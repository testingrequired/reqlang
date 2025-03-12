use std::ops::Range;

/// A pair of T and the span in the original source code
pub type Spanned<T> = (T, Span);

/// A range representing a location in the original source code
pub type Span = Range<usize>;

/// A span representing no location in the original source code
pub const NO_SPAN: Span = 0..0;

#[cfg(test)]
mod tests {
    use super::*;

    pub type StringS = Spanned<String>;

    #[test]
    fn it_works() {
        let spanned_string: StringS = (String::from("test"), 10..15);

        assert_eq!(spanned_string, (String::from("test"), 10..15));
    }
}
