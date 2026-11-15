use crate::SourceSpan;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    ops::Range,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum DiagnosticClass {
    InvalidSyntax,
    Unsupported,
    Semantic,
    NonCanonical,
}

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum DiagnosticCode {
    UnclosedDirective,
    UnexpectedDirectiveClose,
    UnknownDirective,
    DirectiveForm,
    DirectivePlacement,
    NestedDirective,
    AttributeSyntax,
    DuplicateAttribute,
    UnknownAttribute,
    AttributeType,
    MissingAttribute,
    InvalidAttributeValue,
    UnsupportedNode,
    UnsupportedSyntax,
    InvalidInlineSyntax,
    UnclosedCodeFence,
    InvalidCodeFence,
    MissingCodeLanguage,
    UnexpectedCodeMetadata,
    InvalidCodeGroup,
    MissingCodeGroupLabel,
    DuplicateCodeGroupLabel,
    InvalidListNumber,
    InvalidTable,
    UnsafeReference,
    InvalidSvg,
    UnsafeSvg,
    NonCanonicalSource,
    ParserFailure,
    MissingPosition,
    RoundTripMismatch,
    ResourceLimit,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RelatedLocation {
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Diagnostic {
    pub class: DiagnosticClass,
    pub code: DiagnosticCode,
    pub message: String,
    pub span: SourceSpan,
    pub related: Vec<RelatedLocation>,
}

impl Diagnostic {
    pub(crate) fn new(
        class: DiagnosticClass,
        code: DiagnosticCode,
        span: SourceSpan,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            code,
            span,
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub(crate) fn related(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.related.push(RelatedLocation {
            span,
            message: message.into(),
        });
        self
    }

    fn is_invariant(&self) -> bool {
        matches!(
            self.code,
            DiagnosticCode::MissingPosition | DiagnosticCode::ParserFailure
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    pub(crate) fn new(values: Vec<Diagnostic>) -> Self {
        Self(values)
    }

    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructureError {
    Rejected(Diagnostics),
    InvariantViolation(Diagnostic),
}

impl StructureError {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Rejected(diagnostics) => diagnostics.as_slice(),
            Self::InvariantViolation(diagnostic) => std::slice::from_ref(diagnostic),
        }
    }
}

impl From<Diagnostic> for StructureError {
    fn from(diagnostic: Diagnostic) -> Self {
        if diagnostic.is_invariant() {
            Self::InvariantViolation(diagnostic)
        } else {
            Self::Rejected(Diagnostics(vec![diagnostic]))
        }
    }
}

impl fmt::Display for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let diagnostic = &self.diagnostics()[0];
        write!(
            f,
            "{:?} at {}:{}: {}",
            diagnostic.code,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
            diagnostic.message
        )
    }
}

impl Error for StructureError {}

#[derive(Default)]
pub(crate) struct DiagnosticBag {
    values: Vec<Diagnostic>,
    seen: BTreeSet<(usize, usize, DiagnosticCode, String)>,
    suppressed: BTreeMap<usize, usize>,
}

impl DiagnosticBag {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        let range = diagnostic.span.byte_range();
        if !diagnostic.is_invariant()
            && self
                .suppressed
                .range(..=range.start)
                .next_back()
                .is_some_and(|(start, end)| {
                    *start <= range.start && range.start < *end && range.end <= *end
                })
        {
            return;
        }
        let key = (
            range.start,
            range.end,
            diagnostic.code,
            diagnostic.message.clone(),
        );
        if self.seen.insert(key) {
            self.values.push(diagnostic);
        }
    }

    pub fn capture<T>(&mut self, result: Result<T, Diagnostic>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(diagnostic) => {
                self.push(diagnostic);
                None
            }
        }
    }

    pub fn suppress(&mut self, mut range: Range<usize>) {
        if let Some((&start, &end)) = self.suppressed.range(..=range.start).next_back()
            && end >= range.start
        {
            self.suppressed.remove(&start);
            range.start = start;
            range.end = range.end.max(end);
        }
        while let Some((&start, &end)) = self.suppressed.range(range.start..=range.end).next() {
            self.suppressed.remove(&start);
            range.end = range.end.max(end);
        }
        self.suppressed.insert(range.start, range.end);
    }

    pub fn finish(self) -> Result<(), StructureError> {
        let values = self.into_values()?;
        if values.is_empty() {
            Ok(())
        } else {
            Err(StructureError::Rejected(Diagnostics(values)))
        }
    }

    /// 入力の違反を位置の順に返す。エンジン自身の不変条件が破れたときだけ失敗する。
    pub fn into_values(mut self) -> Result<Vec<Diagnostic>, StructureError> {
        self.values
            .sort_by_key(|diagnostic| diagnostic.span.start.byte_offset);
        if let Some(diagnostic) = self
            .values
            .iter()
            .find(|diagnostic| diagnostic.is_invariant())
        {
            return Err(StructureError::InvariantViolation(diagnostic.clone()));
        }
        Ok(self.values)
    }
}
