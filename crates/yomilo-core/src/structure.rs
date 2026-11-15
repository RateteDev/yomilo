use crate::{
    Diagnostic, DiagnosticClass, DiagnosticCode, Diagnostics, InputState, Located, ModelVersion,
    SegmentContent, SegmentError, StructureError, StructuredDocument, SyntaxVersion,
    source::Source,
};

/// 原稿全体を1つの文書として構造化する。セグメントが1つでも規約に反すれば全体を拒否する。
pub fn structure(source: &str) -> Result<StructuredDocument, StructureError> {
    let document = structure_once(source)?;
    let canonical = crate::format(&document);
    let roundtrip = structure_once(&canonical).map_err(|error| {
        StructureError::InvariantViolation(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::RoundTripMismatch,
            Source::new(source).whole_span(),
            format!("Canonical source could not be structured: {error}"),
        ))
    })?;
    if !document.same_meaning(&roundtrip) {
        return Err(StructureError::InvariantViolation(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::RoundTripMismatch,
            Source::new(source).whole_span(),
            "Canonical formatting changed the document meaning",
        )));
    }
    Ok(document)
}

fn structure_once(text: &str) -> Result<StructuredDocument, StructureError> {
    let batch = crate::segment(text, None, InputState::Complete).map_err(|error| match error {
        SegmentError::Structure(error) => error,
        SegmentError::InvalidCursor { message } => {
            unreachable!("the initial cursor is always valid: {message}")
        }
    })?;
    let mut document = StructuredDocument {
        syntax_version: SyntaxVersion::V0,
        model_version: ModelVersion::V0,
        frontmatter: None,
        blocks: Vec::new(),
    };
    let mut rejected = Vec::new();
    for segment in batch.segments {
        match segment.content {
            SegmentContent::Frontmatter(frontmatter) => document.frontmatter = Some(frontmatter),
            SegmentContent::Block(block) => document.blocks.push(Located {
                value: block.value,
                span: block.span,
            }),
            SegmentContent::Rejected(segment) => rejected.extend(segment.diagnostics),
        }
    }
    if rejected.is_empty() {
        Ok(document)
    } else {
        Err(StructureError::Rejected(Diagnostics::new(rejected)))
    }
}

pub fn check_canonical(source: &str) -> Result<(), StructureError> {
    let document = structure(source)?;
    if crate::format(&document) == source {
        return Ok(());
    }
    Err(Diagnostic::new(
        DiagnosticClass::NonCanonical,
        DiagnosticCode::NonCanonicalSource,
        Source::new(source).whole_span(),
        "Source does not match canonical formatting",
    )
    .into())
}
