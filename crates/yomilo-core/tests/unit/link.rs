use yomilo_core::{
    BlockNode, DiagnosticClass, InlineNode, StructureError, StructuredDocument, check_canonical,
    format, structure,
};

fn accepted(input: &str) -> StructuredDocument {
    structure(input).unwrap_or_else(|error| {
        panic!(
            "Explicit link with a URL label was rejected.\nInput:\n{input}\nActual: {error:#?}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    })
}

fn assert_url_label(document: &StructuredDocument, paragraph_index: usize) {
    let BlockNode::Paragraph(paragraph) = &document.blocks()[paragraph_index].value else {
        panic!("Expected a paragraph containing the explicit link");
    };
    let [inline] = paragraph.content.as_slice() else {
        panic!("Expected one explicit link, got: {paragraph:#?}");
    };
    let InlineNode::Link(link) = &inline.value else {
        panic!("URL label was not retained as an explicit link: {inline:#?}");
    };
    assert_eq!(link.href, "https://astro.build/");
    let label: String = link
        .content
        .iter()
        .map(|node| {
            let InlineNode::Text(text) = &node.value else {
                panic!("URL label must remain display text: {node:#?}");
            };
            text.as_str()
        })
        .collect();
    assert_eq!(label, "https://astro.build/");
}

#[test]
fn explicit_link_accepts_and_preserves_a_url_as_its_display_text() {
    let input = "[https://astro.build/](https://astro.build/)\n";
    let document = accepted(input);
    assert_eq!(document.blocks().len(), 1);
    assert_url_label(&document, 0);
}

#[test]
fn url_label_survives_canonical_formatting_and_meaning_preserving_round_trip() {
    let input =
        "# リンクの回帰\n\n[https://astro.build/](https://astro.build/)\n\nリンクの後の本文。\n";
    let before = accepted(input);
    assert_eq!(before.blocks().len(), 3);
    assert_url_label(&before, 1);

    let canonical = format(&before);
    check_canonical(&canonical).unwrap_or_else(|error| {
        panic!(
            "Formatted explicit URL label is not canonical.\nInput:\n{input}\nActual canonical form:\n{canonical}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    });
    let after = accepted(&canonical);
    assert_eq!(after.blocks().len(), 3);
    assert_url_label(&after, 1);
    assert_eq!(format(&after), canonical, "Formatting is not idempotent");
    assert!(
        after.same_meaning(&before),
        "Explicit URL label changed meaning.\nInput:\n{input}\nCanonical:\n{canonical}\nBefore: {before:#?}\nAfter: {after:#?}"
    );
}

#[test]
fn bare_urls_and_autolinks_remain_rejected() {
    let mut failures = Vec::new();
    for input in [
        "https://astro.build/ を見る\n",
        "<https://astro.build/>\n",
        "参照\nhttps://astro.build/ を見る\n",
        "参照\n<https://astro.build/>\n",
        "参照\nwww.example.com\n",
    ] {
        match structure(input) {
            Ok(document) => failures.push(format!(
                "Expected rejection of a URL outside an explicit link.\nInput:\n{input}\nActual success: {document:#?}"
            )),
            Err(error) => {
                if !matches!(error, StructureError::Rejected(_))
                    || error.diagnostics().is_empty()
                    || error
                        .diagnostics()
                        .iter()
                        .any(|diagnostic| diagnostic.class != DiagnosticClass::Unsupported)
                {
                    failures.push(format!(
                        "Expected Unsupported diagnostics.\nInput:\n{input}\nActual: {error:#?}\nDiagnostics: {:#?}",
                        error.diagnostics()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
