use std::collections::HashMap;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use errors::{ParseError, ReqlangError, ResolverError};
use parser::template;
use serde::{Deserialize, Serialize};
use span::Span;
use str_idxpos::index_to_position;

#[derive(Debug, Default)]
pub struct Diagnoser {}

impl Diagnoser {
    pub fn get_diagnostics(source: &str) -> Vec<Diagnosis> {
        match parser::parse(source) {
            Ok(_) => vec![],
            Err(errs) => {
                return errs
                    .iter()
                    .map(|(err, span)| Diagnosis {
                        range: Diagnoser::get_range(source, span),
                        severity: Some(DiagnosisSeverity::ERROR),
                        message: err.to_string(),
                    })
                    .collect();
            }
        }
    }

    pub fn get_diagnostics_with_env(
        source: &str,
        env: &str,
        prompts: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
    ) -> Vec<Diagnosis> {
        match template(source, env, prompts, secrets, HashMap::new()) {
            Ok(_) => vec![],
            Err(errs) => {
                return errs
                    .iter()
                    .map(|(err, span)| Diagnosis {
                        range: Diagnoser::get_range(source, span),
                        severity: Some(DiagnosisSeverity::ERROR),
                        message: err.to_string(),
                    })
                    .collect();
            }
        }
    }

    pub fn get_range(source: &str, span: &Span) -> DiagnosisRange {
        DiagnosisRange {
            start: Diagnoser::get_position(source, span.start),
            end: Diagnoser::get_position(source, span.end),
        }
    }

    pub fn get_position(source: &str, idx: usize) -> DiagnosisPosition {
        let (line, character) = index_to_position(source, idx);

        DiagnosisPosition {
            line: line as u32,
            character: character as u32,
        }
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
    use crate::{Diagnoser, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity};

    #[test]
    fn it_works() {
        let source = String::from("");

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
                message: String::from("ParseError: Request file is an empty file")
            }],
            Diagnoser::get_diagnostics(&source)
        );
    }
}
