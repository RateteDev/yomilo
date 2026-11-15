use std::ops::Range;
use yomilo_core::{Diagnostic, DiagnosticClass, DiagnosticCode, StructureError, structure};

fn rejected(input: &str) -> StructureError {
    let error = match structure(input) {
        Ok(document) => panic!(
            "Expected independent errors to reject the document.\nInput:\n{input}\nActual success: {document:#?}"
        ),
        Err(error) => error,
    };
    assert!(
        matches!(error, StructureError::Rejected(_)),
        "Expected collected user diagnostics, not an invariant violation.\nInput:\n{input}\nActual: {error:#?}"
    );
    error
}

fn assert_position_order(input: &str, diagnostics: &[Diagnostic]) {
    assert!(
        diagnostics
            .windows(2)
            .all(|pair| { pair[0].span.byte_range().start <= pair[1].span.byte_range().start }),
        "Expected diagnostics in source position order.\nInput:\n{input}\nActual: {diagnostics:#?}"
    );
}

fn assert_at_error(
    input: &str,
    diagnostic: &Diagnostic,
    expected_region: Range<usize>,
    expected_class: DiagnosticClass,
) {
    assert!(
        expected_region.contains(&diagnostic.span.byte_range().start),
        "Expected a diagnostic in byte region {expected_region:?}.\nInput:\n{input}\nActual: {diagnostic:#?}"
    );
    assert_eq!(
        diagnostic.class, expected_class,
        "Unexpected classification.\nInput:\n{input}\nActual: {diagnostic:#?}"
    );
}

#[test]
fn independent_unsupported_blocks_return_both_diagnostics_in_document_order() {
    let input = "#### 見出し\n\n![image](figure.png)\n";
    let error = rejected(input);
    let diagnostics = error.diagnostics();
    assert_eq!(
        diagnostics.len(),
        2,
        "Expected both the h4 and Markdown image diagnostics.\nInput:\n{input}\nActual: {diagnostics:#?}"
    );
    assert_position_order(input, diagnostics);
    assert_at_error(input, &diagnostics[0], 0..15, DiagnosticClass::Unsupported);
    assert_at_error(input, &diagnostics[1], 16..37, DiagnosticClass::Unsupported);
}

#[test]
fn independent_attribute_errors_are_all_collected_in_position_order() {
    let input = "::image{src=figure.png ratio=broken mystery=value zoom=wrong}\n";
    let error = rejected(input);
    let diagnostics = error.diagnostics();
    assert!(
        diagnostics.len() >= 3,
        "Expected all three independent errors: invalid ratio, unknown attribute, and a value on a boolean flag.\nInput:\n{input}\nActual: {diagnostics:#?}"
    );
    assert_position_order(input, diagnostics);
    for (region, class) in [
        (23..35, DiagnosticClass::Semantic),
        (36..49, DiagnosticClass::Unsupported),
        (50..60, DiagnosticClass::Semantic),
    ] {
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| region.contains(&diagnostic.span.byte_range().start))
            .unwrap_or_else(|| {
                panic!(
                    "Independent attribute error in byte region {region:?} was lost.\nInput:\n{input}\nActual: {diagnostics:#?}"
                )
            });
        assert_at_error(input, diagnostic, region, class);
    }
}

#[test]
fn unclosed_directive_does_not_hide_an_earlier_unsupported_block() {
    let input = "#### 見出し\n\n:::callout{type=note}\n本文\n";
    let error = rejected(input);
    let diagnostics = error.diagnostics();
    assert_eq!(
        diagnostics.len(),
        2,
        "Expected both the unsupported h4 and the unclosed directive diagnostics.\nInput:\n{input}\nActual: {diagnostics:#?}"
    );
    assert_position_order(input, diagnostics);
    assert_at_error(input, &diagnostics[0], 0..15, DiagnosticClass::Unsupported);
    assert_at_error(
        input,
        &diagnostics[1],
        16..37,
        DiagnosticClass::InvalidSyntax,
    );
    assert_eq!(
        diagnostics[1].code,
        DiagnosticCode::UnclosedDirective,
        "Expected the unclosed directive diagnostic to be retained.\nInput:\n{input}\nActual: {diagnostics:#?}"
    );
}
