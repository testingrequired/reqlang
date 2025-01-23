use codespan_reporting::diagnostic::{Diagnostic, Label};
use errors::{ParseError, ReqlangError, ResolverError};
use serde::{Deserialize, Serialize};
use span::{Span, Spanned};
use str_idxpos::index_to_position;

/// Get a list of diagnostics from a list of errors
pub fn get_diagnostics(errs: &[Spanned<ReqlangError>], source: &str) -> Vec<Diagnosis> {
    errs.iter()
        .map(|(err, span)| Diagnosis {
            range: get_range(source, span),
            severity: Some(DiagnosisSeverity::ERROR),
            message: err.to_string(),
        })
        .collect()
}

fn get_range(source: &str, span: &Span) -> DiagnosisRange {
    DiagnosisRange {
        start: get_position(source, span.start),
        end: get_position(source, span.end),
    }
}

fn get_position(source: &str, idx: usize) -> DiagnosisPosition {
    let (line, character) = index_to_position(source, idx);

    DiagnosisPosition {
        line: line as u32,
        character: character as u32,
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnosis {
    pub range: DiagnosisRange,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<DiagnosisSeverity>,

    pub message: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DiagnosisSeverity(i32);
impl DiagnosisSeverity {
    pub const ERROR: DiagnosisSeverity = DiagnosisSeverity(1);
    pub const WARNING: DiagnosisSeverity = DiagnosisSeverity(2);
    pub const INFORMATION: DiagnosisSeverity = DiagnosisSeverity(3);
    pub const HINT: DiagnosisSeverity = DiagnosisSeverity(4);
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default, Deserialize, Serialize)]
pub struct DiagnosisPosition {
    pub line: u32,
    pub character: u32,
}

impl DiagnosisPosition {
    pub fn new(line: u32, character: u32) -> DiagnosisPosition {
        DiagnosisPosition { line, character }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default, Deserialize, Serialize)]
pub struct DiagnosisRange {
    /// The range's start position (inclusive)
    pub start: DiagnosisPosition,
    /// The range's end position (exclusive)
    pub end: DiagnosisPosition,
}

impl DiagnosisRange {
    pub fn new(start: DiagnosisPosition, end: DiagnosisPosition) -> DiagnosisRange {
        DiagnosisRange { start, end }
    }
}

#[allow(dead_code)]
trait AsDiagnostic {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()>;
}

macro_rules! impl_as_dianostic {
    ($($error:tt),+) => {$(
        impl AsDiagnostic for $error {
            fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
                Diagnostic::error()
                    .with_code(stringify!($error))
                    .with_message(self.to_string())
                    .with_labels(vec![Label::primary((), span.clone())])
            }
        }
    )+};
}

impl_as_dianostic!(ParseError, ResolverError);

impl AsDiagnostic for ReqlangError {
    fn as_diagnostic(&self, span: &Span) -> Diagnostic<()> {
        match self {
            ReqlangError::ParseError(e) => e.as_diagnostic(span),
            ReqlangError::ResolverError(e) => e.as_diagnostic(span),
        }
    }
}

#[cfg(test)]
mod tests {
    use parser::parse;

    use crate::{get_diagnostics, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity};

    #[test]
    fn it_works() {
        let source = String::from("");

        let errs = parse(&source).unwrap_err();

        assert_eq!(
            vec![Diagnosis {
                range: DiagnosisRange {
                    start: DiagnosisPosition {
                        line: 0,
                        character: 0,
                    },
                    end: DiagnosisPosition {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosisSeverity::ERROR),
                message: String::from("ParseError: Request file requires a request be defined")
            }],
            get_diagnostics(&errs, &source)
        );
    }
}
