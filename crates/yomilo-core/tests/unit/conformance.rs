use std::collections::{BTreeMap, BTreeSet};
use yomilo_core::{
    BlockNode, CalloutType, ColumnAlignment, FoldIntent, FrontmatterFormat, HeadingLevel,
    InlineNode, ListKind, Located, StructuredDocument, check_canonical, format, structure,
};

const CATALOG: &str = include_str!("../fixtures/conformance.md");

#[derive(Default, Debug)]
struct Coverage {
    blocks: BTreeSet<&'static str>,
    directives: BTreeMap<&'static str, usize>,
    headings: BTreeMap<&'static str, usize>,
    h3_folds: BTreeSet<&'static str>,
    lists: BTreeSet<&'static str>,
    alignments: BTreeSet<&'static str>,
    inlines: BTreeSet<&'static str>,
    callout_types: BTreeMap<&'static str, usize>,
}

fn increment(counts: &mut BTreeMap<&'static str, usize>, kind: &'static str) {
    *counts.entry(kind).or_default() += 1;
}

fn accepted(input: &str) -> StructuredDocument {
    structure(input).unwrap_or_else(|error| {
        panic!(
            "Conformance input rejected.\nInput:\n{input}\nActual error: {error:#?}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    })
}

fn visible_text(nodes: &[Located<InlineNode>]) -> String {
    nodes
        .iter()
        .map(|node| match &node.value {
            InlineNode::Text(value) | InlineNode::Code(value) => value.clone(),
            InlineNode::Emphasis(content) | InlineNode::Strong(content) => visible_text(content),
            InlineNode::Link(link) => visible_text(&link.content),
        })
        .collect()
}

fn observe_inlines(nodes: &[Located<InlineNode>], coverage: &mut Coverage) {
    for node in nodes {
        let kind = match &node.value {
            InlineNode::Text(_) => "text",
            InlineNode::Emphasis(content) => {
                observe_inlines(content, coverage);
                "emphasis"
            }
            InlineNode::Strong(content) => {
                observe_inlines(content, coverage);
                "strong"
            }
            InlineNode::Code(_) => "inline-code",
            InlineNode::Link(link) => {
                observe_inlines(&link.content, coverage);
                "link"
            }
        };
        coverage.inlines.insert(kind);
    }
}

fn observe_blocks(blocks: &[Located<BlockNode>], coverage: &mut Coverage) {
    // 公開された部品集合の追加時は、網羅的なmatchを更新するまでコンパイルを失敗させる。
    for block in blocks {
        let kind = match &block.value {
            BlockNode::Heading(heading) => {
                let level = match heading.level {
                    HeadingLevel::H1 => "h1",
                    HeadingLevel::H2 => "h2",
                    HeadingLevel::H3 => "h3",
                };
                increment(&mut coverage.headings, level);
                let fold = match heading.fold {
                    FoldIntent::None => "none",
                    FoldIntent::InitiallyClosed => "initially-closed",
                    FoldIntent::InitiallyOpen => "initially-open",
                };
                if heading.level == HeadingLevel::H3 {
                    coverage.h3_folds.insert(fold);
                }
                observe_inlines(&heading.content, coverage);
                "heading"
            }
            BlockNode::Paragraph(paragraph) => {
                observe_inlines(&paragraph.content, coverage);
                "paragraph"
            }
            BlockNode::List(list) => {
                coverage.lists.insert(match list.kind {
                    ListKind::Unordered => "unordered",
                    ListKind::Ordered => "ordered",
                });
                for item in &list.items {
                    observe_blocks(&item.value.blocks, coverage);
                }
                "list"
            }
            BlockNode::Quote(quote) => {
                observe_blocks(&quote.blocks, coverage);
                "quote"
            }
            BlockNode::CodeBlock(_) => "code-block",
            BlockNode::Table(table) => {
                for alignment in &table.align {
                    coverage.alignments.insert(match alignment {
                        ColumnAlignment::Unspecified => "unspecified",
                        ColumnAlignment::Left => "left",
                        ColumnAlignment::Center => "center",
                        ColumnAlignment::Right => "right",
                    });
                }
                for row in std::iter::once(&table.header).chain(table.rows.iter()) {
                    for cell in &row.value.cells {
                        observe_inlines(&cell.value.content, coverage);
                    }
                }
                "table"
            }
            BlockNode::ThematicBreak => "thematic-break",
            BlockNode::Callout(callout) => {
                increment(&mut coverage.directives, "callout");
                increment(
                    &mut coverage.callout_types,
                    match callout.kind {
                        CalloutType::Note => "note",
                        CalloutType::Warning => "warning",
                        CalloutType::Caution => "caution",
                    },
                );
                observe_blocks(&callout.blocks, coverage);
                "callout"
            }
            BlockNode::CodeGroup(_) => {
                increment(&mut coverage.directives, "code-group");
                "code-group"
            }
            BlockNode::Svg(_) => {
                increment(&mut coverage.directives, "svg");
                "svg"
            }
            BlockNode::Image(_) => {
                increment(&mut coverage.directives, "image");
                "image"
            }
            BlockNode::Video(_) => {
                increment(&mut coverage.directives, "video");
                "video"
            }
            BlockNode::UrlCard(_) => {
                increment(&mut coverage.directives, "url-card");
                "url-card"
            }
            BlockNode::TweetCard(_) => {
                increment(&mut coverage.directives, "tweet-card");
                "tweet-card"
            }
        };
        coverage.blocks.insert(kind);
    }
}

fn assert_vocabulary(document: &StructuredDocument) {
    let mut coverage = Coverage::default();
    observe_blocks(document.blocks(), &mut coverage);
    assert_eq!(
        coverage.blocks,
        BTreeSet::from([
            "heading",
            "paragraph",
            "list",
            "quote",
            "code-block",
            "table",
            "thematic-break",
            "callout",
            "code-group",
            "svg",
            "image",
            "video",
            "url-card",
            "tweet-card",
        ])
    );
    assert_eq!(
        coverage.directives,
        BTreeMap::from([
            ("callout", 6),
            ("code-group", 1),
            ("svg", 1),
            ("image", 1),
            ("video", 1),
            ("url-card", 1),
            ("tweet-card", 1),
        ])
    );
    assert_eq!(
        coverage.headings,
        BTreeMap::from([("h1", 1), ("h2", 2), ("h3", 3)])
    );
    assert_eq!(
        coverage.h3_folds,
        BTreeSet::from(["none", "initially-closed", "initially-open"])
    );
    assert_eq!(coverage.lists, BTreeSet::from(["unordered", "ordered"]));
    assert_eq!(
        coverage.alignments,
        BTreeSet::from(["unspecified", "left", "center", "right"])
    );
    assert_eq!(
        coverage.inlines,
        BTreeSet::from(["text", "emphasis", "strong", "inline-code", "link"])
    );
    assert_eq!(
        coverage.callout_types,
        BTreeMap::from([("note", 2), ("warning", 2), ("caution", 2)])
    );
}

fn assert_attribute_values(document: &StructuredDocument) {
    let frontmatter = &document.frontmatter().expect("Frontmatter was lost").value;
    assert_eq!(frontmatter.format, FrontmatterFormat::Yaml);
    assert_eq!(
        frontmatter.value.lines().collect::<Vec<_>>(),
        ["version: 99", "custom: [preserve, opaque]"]
    );
    let BlockNode::Paragraph(paragraph) = &document.blocks()[1].value else {
        panic!("Catalog paragraph was lost");
    };
    assert_eq!(
        visible_text(&paragraph.content),
        "段落では強調、強い強調、inline_code、明示リンクを保持します。"
    );
    let callouts: Vec<_> = document
        .blocks()
        .iter()
        .filter_map(|block| match &block.value {
            BlockNode::Callout(callout) => Some(callout),
            _ => None,
        })
        .collect();
    assert_eq!(callouts.len(), 6);
    for (callout, (kind, label, body)) in callouts.iter().zip([
        (CalloutType::Note, "NOTE", "既定noteの本文。"),
        (CalloutType::Note, "補足 ラベル", "独自noteの本文。"),
        (CalloutType::Warning, "WARNING", "既定warningの本文。"),
        (CalloutType::Warning, "警告 ラベル", "独自warningの本文。"),
        (CalloutType::Caution, "CAUTION", "既定cautionの本文。"),
        (CalloutType::Caution, "注意 ラベル", "独自cautionの本文。"),
    ]) {
        assert_eq!(callout.kind, kind);
        assert_eq!(callout.label, label);
        assert_eq!(callout.blocks.len(), 1);
        let BlockNode::Paragraph(paragraph) = &callout.blocks[0].value else {
            panic!("Callout body was lost");
        };
        assert_eq!(visible_text(&paragraph.content), body);
    }

    for block in document.blocks() {
        match &block.value {
            BlockNode::Table(table) => {
                assert_eq!(
                    table.align,
                    [
                        ColumnAlignment::Unspecified,
                        ColumnAlignment::Left,
                        ColumnAlignment::Center,
                        ColumnAlignment::Right,
                    ]
                );
                assert_eq!(
                    table
                        .header
                        .value
                        .cells
                        .iter()
                        .map(|cell| visible_text(&cell.value.content))
                        .collect::<Vec<_>>(),
                    ["指定なし", "左", "中央", "右"]
                );
                assert_eq!(table.rows.len(), 1);
                assert_eq!(
                    table.rows[0]
                        .value
                        .cells
                        .iter()
                        .map(|cell| visible_text(&cell.value.content))
                        .collect::<Vec<_>>(),
                    ["無指定", "左寄せ", "中央寄せ", "右寄せ"]
                );
            }
            BlockNode::CodeGroup(group) => {
                assert_eq!(
                    group
                        .entries
                        .iter()
                        .map(|entry| (
                            entry.value.label.as_str(),
                            entry.value.code.language.as_str(),
                            entry.value.code.value.as_str()
                        ))
                        .collect::<Vec<_>>(),
                    [
                        ("npm", "bash", "npm install package"),
                        ("pnpm", "bash", "pnpm add package"),
                    ]
                );
            }
            BlockNode::Svg(svg) => {
                assert_eq!(svg.alt.as_deref(), Some("静的 図"));
                assert_eq!(svg.caption.as_deref(), Some("SVGの 説明"));
                assert!(svg.zoom);
                assert_eq!(
                    svg.body.as_str().trim(),
                    "<svg viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\"/></svg>"
                );
            }
            BlockNode::Image(image) => {
                assert_eq!(image.src, "figure.png");
                assert_eq!(image.alt.as_deref(), Some("画像の 代替説明"));
                assert_eq!(image.caption.as_deref(), Some("画像の 説明"));
                let ratio = image.ratio.expect("Image ratio was lost");
                assert_eq!((ratio.width(), ratio.height()), (16, 9));
                assert!(image.zoom);
            }
            BlockNode::Video(video) => {
                assert_eq!(video.src, "clip.mp4");
                assert_eq!(video.caption.as_deref(), Some("動画の 説明"));
                let ratio = video.ratio.expect("Video ratio was lost");
                assert_eq!((ratio.width(), ratio.height()), (4, 3));
                assert!(video.loop_playback);
            }
            BlockNode::UrlCard(card) => {
                assert_eq!(card.href, "https://example.com/article");
            }
            BlockNode::TweetCard(card) => {
                assert_eq!(card.href, "https://x.com/example/status/1");
            }
            _ => {}
        }
    }
}

#[test]
fn catalog_structures_and_covers_the_supported_vocabulary_and_attributes() {
    let document = accepted(CATALOG);
    assert_vocabulary(&document);
    assert_attribute_values(&document);

    let headers: Vec<_> = document
        .blocks()
        .iter()
        .filter(|block| matches!(block.value, BlockNode::Callout(_)))
        .map(|block| {
            CATALOG[block.span.byte_range()]
                .lines()
                .next()
                .expect("Callout source span has no opening line")
        })
        .collect();
    assert_eq!(
        headers,
        [
            ":::callout{type=\"note\" label=\"NOTE\"}",
            ":::callout{type=\"note\" label=\"補足 ラベル\"}",
            ":::callout{type=\"warning\" label=\"WARNING\"}",
            ":::callout{type=\"warning\" label=\"警告 ラベル\"}",
            ":::callout{type=\"caution\" label=\"CAUTION\"}",
            ":::callout{type=\"caution\" label=\"注意 ラベル\"}",
        ],
        "Catalog must contain both default and custom labels explicitly for all three Callout types"
    );
}

#[test]
fn catalog_is_canonical() {
    let document = accepted(CATALOG);
    check_canonical(CATALOG).unwrap_or_else(|error| {
        panic!(
            "Conformance catalog is not canonical.\nExpected input:\n{CATALOG}\nActual canonical form:\n{}\nDiagnostics: {:#?}",
            format(&document),
            error.diagnostics()
        )
    });
    assert_eq!(format(&document), CATALOG);
}

#[test]
fn catalog_format_is_idempotent_and_round_trip_preserves_meaning() {
    let before = accepted(CATALOG);
    assert_vocabulary(&before);
    assert_attribute_values(&before);
    let canonical = format(&before);
    let after = accepted(&canonical);
    assert_vocabulary(&after);
    assert_attribute_values(&after);
    assert_eq!(format(&after), canonical);
    assert!(
        after.same_meaning(&before),
        "Conformance round trip changed meaning.\nBefore: {before:#?}\nAfter: {after:#?}"
    );
    check_canonical(&canonical).expect("Formatted catalog was rejected as noncanonical");
}

fn assert_callout_normalization(
    input: &str,
    expected_kind: CalloutType,
    expected_label: &str,
    expected_canonical: &str,
) {
    let before = accepted(input);
    assert_eq!(before.blocks().len(), 1);
    let BlockNode::Callout(callout) = &before.blocks()[0].value else {
        panic!("Expected Callout for omitted attributes");
    };
    assert_eq!(callout.kind, expected_kind);
    assert_eq!(callout.label, expected_label);
    assert_eq!(callout.blocks.len(), 1);
    let BlockNode::Paragraph(paragraph) = &callout.blocks[0].value else {
        panic!("Callout body was lost");
    };
    assert_eq!(visible_text(&paragraph.content), "本文");

    let canonical = format(&before);
    assert_eq!(canonical, expected_canonical, "Input:\n{input}");
    check_canonical(&canonical).unwrap_or_else(|error| {
        panic!(
            "Explicit defaults were rejected as noncanonical.\nInput:\n{input}\nExpected:\n{expected_canonical}\nActual:\n{canonical}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    });
    let after = accepted(&canonical);
    assert!(after.same_meaning(&before));
    assert_eq!(format(&after), expected_canonical);
}

#[test]
fn callout_label_omission_normalizes_to_explicit_defaults() {
    for (input, kind, label, canonical) in [
        (
            ":::callout{type=note}\n本文\n:::\n",
            CalloutType::Note,
            "NOTE",
            ":::callout{type=\"note\" label=\"NOTE\"}\n本文\n:::\n",
        ),
        (
            ":::callout{type=warning}\n本文\n:::\n",
            CalloutType::Warning,
            "WARNING",
            ":::callout{type=\"warning\" label=\"WARNING\"}\n本文\n:::\n",
        ),
        (
            ":::callout{type=caution}\n本文\n:::\n",
            CalloutType::Caution,
            "CAUTION",
            ":::callout{type=\"caution\" label=\"CAUTION\"}\n本文\n:::\n",
        ),
    ] {
        assert_callout_normalization(input, kind, label, canonical);
    }
}

#[test]
fn callout_type_omission_normalizes_to_explicit_note_defaults() {
    assert_callout_normalization(
        ":::callout{}\n本文\n:::\n",
        CalloutType::Note,
        "NOTE",
        ":::callout{type=\"note\" label=\"NOTE\"}\n本文\n:::\n",
    );
}
