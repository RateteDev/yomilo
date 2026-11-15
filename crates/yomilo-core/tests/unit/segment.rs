use yomilo_core::{
    BlockNode, Cursor, DiagnosticCode, InputState, Segment, SegmentContent, SegmentError, segment,
};

const CONFORMANCE: &str = include_str!("../fixtures/conformance.md");

fn complete(source: &str) -> Vec<Segment> {
    segment(source, None, InputState::Complete)
        .unwrap_or_else(|error| panic!("Segmentation failed.\nInput:\n{source}\n{error:#?}"))
        .segments
}

fn kinds(segments: &[Segment]) -> Vec<&'static str> {
    segments
        .iter()
        .map(|segment| match &segment.content {
            SegmentContent::Frontmatter(_) => "frontmatter",
            SegmentContent::Rejected(_) => "rejected",
            SegmentContent::Block(block) => match block.value {
                BlockNode::Heading(_) => "heading",
                BlockNode::Paragraph(_) => "paragraph",
                BlockNode::List(_) => "list",
                BlockNode::Table(_) => "table",
                BlockNode::CodeBlock(_) => "code",
                BlockNode::Callout(_) => "callout",
                _ => "other",
            },
        })
        .collect()
}

/// 原稿を少しずつ伸ばしながら渡し、確定したセグメントをつなげる。
fn streamed(source: &str, step: usize) -> Vec<Segment> {
    let mut cursor: Option<Cursor> = None;
    let mut segments = Vec::new();
    let mut end = 0;
    while end < source.len() {
        end = (end + step).min(source.len());
        while !source.is_char_boundary(end) {
            end += 1;
        }
        let batch = segment(&source[..end], cursor.as_ref(), InputState::Partial)
            .unwrap_or_else(|error| panic!("Partial input failed at {end}.\n{error:#?}"));
        segments.extend(batch.segments);
        cursor = Some(batch.cursor);
    }
    let batch = segment(source, cursor.as_ref(), InputState::Complete)
        .unwrap_or_else(|error| panic!("Completing the stream failed.\n{error:#?}"));
    segments.extend(batch.segments);
    segments
}

#[test]
fn streamed_input_confirms_the_same_segments_as_the_complete_source() {
    // 空行の後の`2`は、`.`が届くまで段落に見えるが、届くと前のリストへ加わる。
    let violations = "## 節\n\nこれは**重要。**次の文。\n\n1. 項目\n\n2. 項目\n\n| a | b |\n|---|---|\n| 1 |\n\n```rust\nfn main() {}\n```\n\n### 小節\n";
    for source in [CONFORMANCE, violations] {
        let expected = complete(source);
        for step in [1, 3, 17] {
            assert_eq!(
                streamed(source, step),
                expected,
                "Streaming by {step} bytes changed confirmed segments.\nInput:\n{source}"
            );
        }
    }
}

#[test]
fn a_violation_rejects_only_its_own_segment_with_its_source_and_position() {
    let source = "## 節\n\n前の段落。\n\nこれは**重要。**次の文。\n\n- 後の項目\n";
    let segments = complete(source);
    assert_eq!(
        kinds(&segments),
        ["heading", "paragraph", "rejected", "list"]
    );
    let SegmentContent::Rejected(rejected) = &segments[2].content else {
        unreachable!();
    };
    assert_eq!(rejected.source, "これは**重要。**次の文。");
    assert_eq!(rejected.diagnostics.len(), 1);
    assert_eq!(
        rejected.diagnostics[0].code,
        DiagnosticCode::InvalidInlineSyntax
    );
    assert_eq!(rejected.diagnostics[0].span.start.line, 5);
}

#[test]
fn partial_input_leaves_the_growing_tail_pending_until_completed() {
    let source = "前の段落。\n\n```rust\nfn main() {\n";
    let batch = segment(source, None, InputState::Partial).expect("Partial input failed");
    assert_eq!(kinds(&batch.segments), ["paragraph"]);
    assert_eq!(batch.pending, "```rust\nfn main() {\n");

    let batch = segment(source, Some(&batch.cursor), InputState::Complete)
        .expect("Completing the input failed");
    assert_eq!(kinds(&batch.segments), ["rejected"]);
    assert_eq!(batch.pending, "");
}

#[test]
fn a_cursor_from_another_source_is_refused_instead_of_misreading() {
    let batch = segment("一つ目の段落。\n\n二つ目\n", None, InputState::Partial)
        .expect("Partial input failed");
    let error = segment("短い", Some(&batch.cursor), InputState::Partial)
        .expect_err("A cursor beyond the source was accepted");
    assert!(
        matches!(error, SegmentError::InvalidCursor { .. }),
        "{error:#?}"
    );
}
