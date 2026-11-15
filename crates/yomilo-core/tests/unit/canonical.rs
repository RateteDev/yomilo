use yomilo_core::{check_canonical, format, structure};

#[test]
fn canonical_format_has_no_trailing_whitespace_and_keeps_negative_table_cells_unescaped() {
    let input = "> 引用\n> \n> - 項目\n\n| 値 | 差分 |\n| --- | --- |\n| -12.9% | -1 |\n";
    let before = structure(input).unwrap_or_else(|error| {
        panic!("Expected valid quote and table.\nInput:\n{input}\nActual: {error:#?}")
    });
    let canonical = format(&before);
    for (index, line) in canonical.lines().enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "Canonical line {} has trailing whitespace.\nInput:\n{input}\nActual:\n{canonical}",
            index + 1
        );
    }
    assert!(canonical.contains("> 引用\n>\n> - 項目\n"), "{canonical}");
    assert!(canonical.contains("| -12.9% | -1 |\n"), "{canonical}");
    assert!(!canonical.contains("\\-12.9%"), "{canonical}");
    assert!(!canonical.contains("\\-1"), "{canonical}");
    check_canonical(&canonical).unwrap_or_else(|error| {
        panic!(
            "Formatted source is not canonical.\nActual:\n{canonical}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    });
    let after = structure(&canonical).expect("Canonical source was rejected");
    assert!(after.same_meaning(&before));
    assert_eq!(format(&after), canonical);
}

#[test]
fn canonical_soft_breaks_protect_continuation_syntax_and_adjacent_spaces() {
    for source in [
        "前\n\\- 項目\n",
        "前\n\\+ 項目\n",
        "前\n1\\. 項目\n",
        "前\n1\\) 項目\n",
        "前\n\\===\n",
        "前 &#32;\n&#32;後\n",
    ] {
        let before = structure(source).unwrap_or_else(|error| {
            panic!(
                "Continuation text must remain a paragraph.\nInput:\n{source}\nActual: {error:#?}"
            )
        });
        let canonical = format(&before);
        assert_eq!(canonical, source);
        let after = structure(&canonical).expect("Canonical continuation was rejected");
        assert!(after.same_meaning(&before));
        assert_eq!(format(&after), canonical);
        check_canonical(&canonical).expect("Continuation text must remain canonical");
    }
}

#[test]
fn newline_entities_do_not_create_blocks_or_split_single_line_constructs() {
    for source in [
        "&#10;前&#10;\n",
        "前&#10;&#10;後\n",
        "## 前&#10;後\n",
        "| 前&#10;後 |\n| --- |\n",
    ] {
        let before = structure(source).unwrap_or_else(|error| {
            panic!("Newline entities must remain text.\nInput:\n{source}\nActual: {error:#?}")
        });
        let canonical = format(&before);
        assert_eq!(canonical, source);
        let after = structure(&canonical).expect("Canonical newline entities were rejected");
        assert!(after.same_meaning(&before));
        assert_eq!(format(&after), canonical);
        check_canonical(&canonical).expect("Newline entities must remain canonical");
    }
}
