use std::collections::HashSet;
use yomilo_core::{
    BlockNode, CalloutType, ColumnAlignment, DiagnosticClass, FoldIntent, FrontmatterFormat,
    HeadingLevel, InlineNode, ListKind, Located, ModelVersion, StructureError, StructuredDocument,
    SyntaxVersion, check_canonical, format, structure,
};

const MARKDOWN: &str = r#"---
version: 99
custom: [uninterpreted, yes]
---

# 記事

## 節

残す本文: **強調**と*斜体*と`code`と[リンク](https://example.com/article)。

- 最初
- 次

1. 一番
2. 二番

> 引用本文
>
> - 引用内の項目

```rust
println!("kept");
```

| 名前 | 値 |
| :--- | ---: |
| 項目 | 保持 |

---

### > 閉じた補足

補足本文
"#;

const COMPONENTS: &str = r#":::callout
既定の本文
:::

:::callout{type=warning}
警告本文
:::

:::callout{type=caution label="独自 ラベル"}
- 注意事項
:::

:::code-group
```bash npm
npm install package
```
```bash pnpm
pnpm add package
```
:::

:::svg{alt="静的 図" caption="図の 説明" zoom}
<svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>
:::

::image{src=figure.png alt="図の 説明" ratio=16/9 caption="画像の 説明" zoom}

::video{src=clip.mp4 ratio=4/3 caption="動画の 説明" loop}

::url-card{href=https://example.com/article}

::tweet-card{href=https://x.com/example/status/1}
"#;

fn accepted(input: &str) -> StructuredDocument {
    structure(input).unwrap_or_else(|error| {
        panic!(
            "Expected valid strict Markdown.\nInput:\n{input}\nActual error: {error:?}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    })
}

fn visible_text(nodes: &[Located<InlineNode>]) -> String {
    nodes
        .iter()
        .map(|node| match &node.value {
            InlineNode::Text(text) | InlineNode::Code(text) => text.clone(),
            InlineNode::Emphasis(content) | InlineNode::Strong(content) => visible_text(content),
            InlineNode::Link(link) => visible_text(&link.content),
        })
        .collect()
}

fn paragraph_text(block: &Located<BlockNode>) -> String {
    let BlockNode::Paragraph(paragraph) = &block.value else {
        panic!("Expected a paragraph, got: {block:?}");
    };
    visible_text(&paragraph.content)
}

#[test]
fn paragraph_soft_breaks_keep_newlines_in_japanese_and_ascii_text() {
    for (input, expected) in [
        ("一行目\n二行目\n", "一行目\n二行目"),
        ("first 123\nsecond 456\n", "first 123\nsecond 456"),
        ("一行目\r\n二行目\r\n", "一行目\n二行目"),
        ("first\rsecond\r", "first\nsecond"),
    ] {
        let before = accepted(input);
        assert_eq!(before.blocks().len(), 1, "Input: {input:?}");
        let BlockNode::Paragraph(paragraph) = &before.blocks()[0].value else {
            panic!("Expected a paragraph: {before:#?}");
        };
        let [node] = paragraph.content.as_slice() else {
            panic!("Expected one text node: {paragraph:#?}");
        };
        assert_eq!(node.value, InlineNode::Text(expected.to_owned()));
        assert_eq!(node.span.start.line, 1);
        assert_eq!(node.span.end.line, 2);
        let canonical = format(&before);
        assert_eq!(canonical, format!("{expected}\n"));
        let after = accepted(&canonical);
        assert!(after.same_meaning(&before));
        assert_eq!(format(&after), canonical);
        check_canonical(&canonical).expect("Soft breaks must remain canonical");
    }
}

#[test]
fn paragraph_soft_breaks_survive_inline_boundaries_and_link_labels() {
    let input = "前\n**強い\n強調**\n*斜体\n文字*\n[表示\nhttps://example.com/](https://example.com/)\n`code\nspan`\n後\n";
    let before = accepted(input);
    assert_eq!(before.blocks().len(), 1);
    let BlockNode::Paragraph(paragraph) = &before.blocks()[0].value else {
        panic!("Expected a paragraph: {before:#?}");
    };
    let meanings: Vec<_> = paragraph
        .content
        .iter()
        .map(|node| match &node.value {
            InlineNode::Text(value) => ("text", value.clone()),
            InlineNode::Strong(children) => ("strong", visible_text(children)),
            InlineNode::Emphasis(children) => ("emphasis", visible_text(children)),
            InlineNode::Link(link) => {
                assert_eq!(link.href, "https://example.com/");
                ("link", visible_text(&link.content))
            }
            InlineNode::Code(value) => ("code", value.clone()),
        })
        .collect();
    assert_eq!(
        meanings,
        [
            ("text", "前\n".to_owned()),
            ("strong", "強い\n強調".to_owned()),
            ("text", "\n".to_owned()),
            ("emphasis", "斜体\n文字".to_owned()),
            ("text", "\n".to_owned()),
            ("link", "表示\nhttps://example.com/".to_owned()),
            ("text", "\n".to_owned()),
            ("code", "code span".to_owned()),
            ("text", "\n後".to_owned()),
        ]
    );
    let canonical = format(&before);
    assert!(canonical.starts_with("前\n**強い\n強調**\n*斜体\n文字*\n"));
    let after = accepted(&canonical);
    assert!(after.same_meaning(&before));
    assert_eq!(format(&after), canonical);
    check_canonical(&canonical).expect("Inline soft breaks must remain canonical");
}

fn assert_rejected_cases(
    cases: &[(&str, &str, DiagnosticClass)],
    alternative_classes: &[DiagnosticClass],
) {
    let mut failures = Vec::new();
    for (name, input, expected_class) in cases {
        match structure(input) {
            Ok(document) => failures.push(format!(
                "Case: {name}\nInput:\n{input}\nExpected rejection ({expected_class:?}, alternatives: {alternative_classes:?}).\nActual success: {document:#?}"
            )),
            Err(error) => {
                let diagnostics = error.diagnostics();
                if !matches!(error, StructureError::Rejected(_))
                    || diagnostics.is_empty()
                    || (diagnostics[0].class != *expected_class
                        && !alternative_classes.contains(&diagnostics[0].class))
                {
                    failures.push(format!(
                        "Case: {name}\nInput:\n{input}\nExpected rejection ({expected_class:?}, alternatives: {alternative_classes:?}).\nActual error: {error:?}\nDiagnostics: {diagnostics:#?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn canonical_format_is_idempotent() {
    let cases = [
        ("Markdown", MARKDOWN),
        ("all seven components", COMPONENTS),
        ("ordinary fragment", "保持する本文\n\n\n次の段落"),
        ("literal fence inside code", "````text\n```\n:::\n````\n"),
    ];
    let mut failures = Vec::new();
    for (name, input) in cases {
        let result = structure(input).and_then(|document| {
            let once = format(&document);
            let twice = format(&structure(&once)?);
            if once != twice {
                failures.push(format!(
                    "Case: {name}\nInput:\n{input}\nF(x):\n{once}\nF(F(x)):\n{twice}"
                ));
            }
            if let Err(error) = check_canonical(&once) {
                failures.push(format!(
                    "Case: {name}\nExpected F(x) to be canonical.\nF(x):\n{once}\nActual error: {error:#?}"
                ));
            }
            Ok(())
        });
        if let Err(error) = result {
            failures.push(format!(
                "Case: {name}\nInput:\n{input}\nExpected a successful round trip.\nActual error: {error:#?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn assert_markdown_meaning(document: &StructuredDocument) {
    assert_eq!(document.syntax_version(), SyntaxVersion::V0);
    assert_eq!(document.model_version(), ModelVersion::V0);
    let frontmatter = &document.frontmatter().expect("Frontmatter was lost").value;
    assert_eq!(frontmatter.format, FrontmatterFormat::Yaml);
    assert_eq!(
        frontmatter.value.lines().collect::<Vec<_>>(),
        ["version: 99", "custom: [uninterpreted, yes]"]
    );

    let blocks = document.blocks();
    assert_eq!(blocks.len(), 11, "A Markdown block was lost or added");
    let BlockNode::Heading(title) = &blocks[0].value else {
        panic!("Article title was lost");
    };
    assert_eq!(title.level, HeadingLevel::H1);
    assert_eq!(visible_text(&title.content), "記事");
    let BlockNode::Heading(section) = &blocks[1].value else {
        panic!("Section heading was lost");
    };
    assert_eq!(section.level, HeadingLevel::H2);
    assert_eq!(visible_text(&section.content), "節");

    let BlockNode::Paragraph(paragraph) = &blocks[2].value else {
        panic!("Main paragraph was lost");
    };
    assert_eq!(
        visible_text(&paragraph.content),
        "残す本文: 強調と斜体とcodeとリンク。"
    );
    let inline_meanings: Vec<_> = paragraph
        .content
        .iter()
        .filter_map(|node| match &node.value {
            InlineNode::Strong(content) => Some(("strong", visible_text(content), None)),
            InlineNode::Emphasis(content) => Some(("emphasis", visible_text(content), None)),
            InlineNode::Code(value) => Some(("code", value.clone(), None)),
            InlineNode::Link(link) => Some((
                "link",
                visible_text(&link.content),
                Some(link.href.as_str()),
            )),
            InlineNode::Text(_) => None,
        })
        .collect();
    assert_eq!(
        inline_meanings,
        [
            ("strong", "強調".into(), None),
            ("emphasis", "斜体".into(), None),
            ("code", "code".into(), None),
            ("link", "リンク".into(), Some("https://example.com/article")),
        ]
    );

    for (index, kind, expected_items) in [
        (3, ListKind::Unordered, ["最初", "次"]),
        (4, ListKind::Ordered, ["一番", "二番"]),
    ] {
        let BlockNode::List(list) = &blocks[index].value else {
            panic!("Expected list at block {index}");
        };
        assert_eq!(list.kind, kind);
        assert_eq!(list.items.len(), 2);
        for (item, text) in list.items.iter().zip(expected_items) {
            assert_eq!(item.value.blocks.len(), 1);
            assert_eq!(paragraph_text(&item.value.blocks[0]), text);
        }
    }

    let BlockNode::Quote(quote) = &blocks[5].value else {
        panic!("Quote was lost");
    };
    assert_eq!(quote.blocks.len(), 2);
    assert_eq!(paragraph_text(&quote.blocks[0]), "引用本文");
    let BlockNode::List(list) = &quote.blocks[1].value else {
        panic!("List inside quote was lost");
    };
    assert_eq!(list.kind, ListKind::Unordered);
    assert_eq!(list.items.len(), 1);
    assert_eq!(
        paragraph_text(&list.items[0].value.blocks[0]),
        "引用内の項目"
    );

    let BlockNode::CodeBlock(code) = &blocks[6].value else {
        panic!("Code was lost");
    };
    assert_eq!(code.language, "rust");
    assert_eq!(code.value, "println!(\"kept\");");
    let BlockNode::Table(table) = &blocks[7].value else {
        panic!("Table was lost");
    };
    assert_eq!(table.align, [ColumnAlignment::Left, ColumnAlignment::Right]);
    assert_eq!(
        table
            .header
            .value
            .cells
            .iter()
            .map(|cell| visible_text(&cell.value.content))
            .collect::<Vec<_>>(),
        ["名前", "値"]
    );
    assert_eq!(table.rows.len(), 1);
    assert_eq!(
        table.rows[0]
            .value
            .cells
            .iter()
            .map(|cell| visible_text(&cell.value.content))
            .collect::<Vec<_>>(),
        ["項目", "保持"]
    );
    assert!(matches!(blocks[8].value, BlockNode::ThematicBreak));
    let BlockNode::Heading(folded) = &blocks[9].value else {
        panic!("Folded heading was lost");
    };
    assert_eq!(folded.level, HeadingLevel::H3);
    assert_eq!(folded.fold, FoldIntent::InitiallyClosed);
    assert_eq!(visible_text(&folded.content), "閉じた補足");
    assert_eq!(paragraph_text(&blocks[10]), "補足本文");
}

fn assert_component_meaning(document: &StructuredDocument) {
    let blocks = document.blocks();
    assert_eq!(blocks.len(), 9, "A component was lost or added");
    for (index, kind, label, body) in [
        (0, CalloutType::Note, "NOTE", "既定の本文"),
        (1, CalloutType::Warning, "WARNING", "警告本文"),
    ] {
        let BlockNode::Callout(callout) = &blocks[index].value else {
            panic!("Expected callout at block {index}");
        };
        assert_eq!(callout.kind, kind);
        assert_eq!(callout.label, label);
        assert_eq!(callout.blocks.len(), 1);
        assert_eq!(paragraph_text(&callout.blocks[0]), body);
    }
    let BlockNode::Callout(caution) = &blocks[2].value else {
        panic!("Caution callout was lost");
    };
    assert_eq!(caution.kind, CalloutType::Caution);
    assert_eq!(caution.label, "独自 ラベル");
    assert_eq!(caution.blocks.len(), 1);
    let BlockNode::List(list) = &caution.blocks[0].value else {
        panic!("Callout list was lost");
    };
    assert_eq!(list.items.len(), 1);
    assert_eq!(paragraph_text(&list.items[0].value.blocks[0]), "注意事項");

    let BlockNode::CodeGroup(group) = &blocks[3].value else {
        panic!("CodeGroup was lost");
    };
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
            ("pnpm", "bash", "pnpm add package")
        ]
    );
    let BlockNode::Svg(svg) = &blocks[4].value else {
        panic!("SVG was lost");
    };
    assert_eq!(
        svg.body.as_str().trim(),
        "<svg viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\"/></svg>"
    );
    assert_eq!(svg.alt.as_deref(), Some("静的 図"));
    assert_eq!(svg.caption.as_deref(), Some("図の 説明"));
    assert!(svg.zoom);
    let BlockNode::Image(image) = &blocks[5].value else {
        panic!("Image was lost");
    };
    assert_eq!(image.src, "figure.png");
    assert_eq!(image.alt.as_deref(), Some("図の 説明"));
    assert_eq!(image.caption.as_deref(), Some("画像の 説明"));
    let ratio = image.ratio.expect("Image ratio was lost");
    assert_eq!((ratio.width(), ratio.height()), (16, 9));
    assert!(image.zoom);
    let BlockNode::Video(video) = &blocks[6].value else {
        panic!("Video was lost");
    };
    assert_eq!(video.src, "clip.mp4");
    assert_eq!(video.caption.as_deref(), Some("動画の 説明"));
    let ratio = video.ratio.expect("Video ratio was lost");
    assert_eq!((ratio.width(), ratio.height()), (4, 3));
    assert!(video.loop_playback);
    let BlockNode::UrlCard(card) = &blocks[7].value else {
        panic!("URL card was lost");
    };
    assert_eq!(card.href, "https://example.com/article");
    let BlockNode::TweetCard(card) = &blocks[8].value else {
        panic!("Tweet card was lost");
    };
    assert_eq!(card.href, "https://x.com/example/status/1");
}

#[test]
fn round_trip_keeps_handwritten_body_frontmatter_and_component_meanings() {
    for (input, assert_meaning) in [
        (MARKDOWN, assert_markdown_meaning as fn(&StructuredDocument)),
        (COMPONENTS, assert_component_meaning),
    ] {
        let before = accepted(input);
        assert_meaning(&before);
        let canonical = format(&before);
        let after = accepted(&canonical);
        assert_meaning(&after);
        assert!(
            after.same_meaning(&before),
            "Round trip changed meaning.\nInput:\n{input}\nCanonical:\n{canonical}\nBefore: {before:#?}\nAfter: {after:#?}"
        );
    }
}

#[test]
fn unsupported_markdown_is_rejected_not_silently_rendered() {
    use DiagnosticClass::Unsupported;
    assert_rejected_cases(
        &[
            ("footnote", "本文[^1]\n\n[^1]: 注釈\n", Unsupported),
            ("checked task", "- [x] 完了\n", Unsupported),
            ("unchecked task", "- [ ] 未完了\n", Unsupported),
            (
                "reference link",
                "[文字][target]\n\n[target]: https://example.com/\n",
                Unsupported,
            ),
            (
                "defined shortcut reference",
                "[文字]\n\n[文字]: https://example.com/\n",
                Unsupported,
            ),
            ("Setext h1", "見出し\n===\n", Unsupported),
            ("Setext h2", "見出し\n---\n", Unsupported),
            ("h4", "#### 見出し\n", Unsupported),
            ("h6", "###### 見出し\n", Unsupported),
            ("Markdown image", "![説明](figure.png)\n", Unsupported),
            (
                "bare URL",
                "参照 https://example.com/article\n",
                Unsupported,
            ),
            ("strikethrough", "~~削除された本文~~\n", Unsupported),
            ("space hard break", "前  \n後\n", Unsupported),
            ("backslash hard break", "前\\\n後\n", Unsupported),
        ],
        &[],
    );
}

#[test]
fn undefined_shortcut_brackets_remain_plain_text() {
    let document = accepted("定義のない[文字]は本文です。\n");
    assert_eq!(document.blocks().len(), 1);
    let BlockNode::Paragraph(paragraph) = &document.blocks()[0].value else {
        panic!("Expected plain paragraph");
    };
    assert!(
        paragraph
            .content
            .iter()
            .all(|node| matches!(node.value, InlineNode::Text(_)))
    );
    assert_eq!(
        visible_text(&paragraph.content),
        "定義のない[文字]は本文です。"
    );
    let after = accepted(&format(&document));
    assert_eq!(
        paragraph_text(&after.blocks()[0]),
        "定義のない[文字]は本文です。"
    );
    assert!(after.same_meaning(&document));
}

#[test]
fn directive_markers_inside_fences_remain_literal_code() {
    let body = ":::\n:::callout{type=warning}\nkept inside code\n:::";
    let inputs = [
        ("ordinary code", format!("```text\n{body}\n```\n")),
        (
            "CodeGroup code",
            format!(":::code-group\n```text literal\n{body}\n```\n:::\n"),
        ),
        (
            "Callout code",
            format!(":::callout\n```text\n{body}\n```\n:::\n"),
        ),
    ];
    for (name, input) in inputs {
        let before = accepted(&input);
        let canonical = format(&before);
        let after = accepted(&canonical);
        for document in [&before, &after] {
            assert_eq!(document.blocks().len(), 1, "{name}");
            let code = match &document.blocks()[0].value {
                BlockNode::CodeBlock(code) => code,
                BlockNode::CodeGroup(group) => {
                    assert_eq!(group.entries.len(), 1);
                    assert_eq!(group.entries[0].value.label, "literal");
                    &group.entries[0].value.code
                }
                BlockNode::Callout(callout) => {
                    assert_eq!(callout.blocks.len(), 1);
                    let BlockNode::CodeBlock(code) = &callout.blocks[0].value else {
                        panic!("Code inside callout was lost");
                    };
                    code
                }
                other => panic!("Expected code container, got: {other:?}"),
            };
            assert_eq!(code.language, "text", "{name}");
            assert_eq!(code.value, body, "{name}\nInput:\n{input}");
        }
        assert!(after.same_meaning(&before), "{name}");
    }
}

#[test]
fn unclosed_directives_point_to_the_opening_marker() {
    for (marker, input) in [
        (":::callout", "前置き\n\n:::callout\n本文\n"),
        (
            ":::code-group",
            "前置き\n\n:::code-group\n```text label\ncode\n```\n",
        ),
        (
            ":::svg",
            "前置き\n\n:::svg\n<svg viewBox=\"0 0 10 10\"></svg>\n",
        ),
    ] {
        let error = structure(input).expect_err("Unclosed directive was accepted");
        assert!(matches!(error, StructureError::Rejected(_)), "{error:?}");
        let diagnostic = error.diagnostics().first().expect("Missing diagnostic");
        assert_eq!(
            diagnostic.class,
            DiagnosticClass::InvalidSyntax,
            "{error:?}"
        );
        let range = diagnostic.span.byte_range();
        let opening = input.find(marker).expect("Invalid test fixture");
        assert_eq!(
            range.start, opening,
            "Expected diagnostic at opening marker.\nInput:\n{input}\nActual: {error:#?}"
        );
        assert!(
            range.end >= opening + 3 && range.end <= input.len(),
            "Expected diagnostic span to cover opening colons: {error:#?}"
        );
    }
}

#[test]
fn unknown_and_wrong_form_directives_are_rejected() {
    use DiagnosticClass::{InvalidSyntax, Unsupported};
    assert_rejected_cases(
        &[
            ("unknown block", ":::mystery\n本文\n:::\n", Unsupported),
            (
                "unknown single line",
                "::mystery{src=figure.png}\n",
                Unsupported,
            ),
            ("single-line Callout", "::callout\n", InvalidSyntax),
            ("single-line CodeGroup", "::code-group\n", InvalidSyntax),
            ("single-line SVG", "::svg\n", InvalidSyntax),
            (
                "block Image",
                ":::image{src=figure.png}\n本文\n:::\n",
                InvalidSyntax,
            ),
            (
                "block Video",
                ":::video{src=clip.mp4}\n本文\n:::\n",
                InvalidSyntax,
            ),
            (
                "block URL card",
                ":::url-card{href=https://example.com/}\n本文\n:::\n",
                InvalidSyntax,
            ),
            (
                "block Tweet card",
                ":::tweet-card{href=https://x.com/example/status/1}\n本文\n:::\n",
                InvalidSyntax,
            ),
        ],
        &[Unsupported],
    );
}

#[test]
fn directives_are_standalone_top_level_and_not_nested() {
    use DiagnosticClass::{InvalidSyntax, Unsupported};
    assert_rejected_cases(
        &[
            (
                "no preceding blank line",
                "本文\n::image{src=figure.png}\n",
                InvalidSyntax,
            ),
            (
                "no following blank line",
                "::image{src=figure.png}\n本文\n",
                InvalidSyntax,
            ),
            (
                "inline placement",
                "本文 ::image{src=figure.png}\n",
                InvalidSyntax,
            ),
            ("inside quote", "> ::image{src=figure.png}\n", InvalidSyntax),
            ("inside list", "- ::image{src=figure.png}\n", InvalidSyntax),
            (
                "nested directive",
                ":::callout\n\n::image{src=figure.png}\n\n:::\n",
                Unsupported,
            ),
        ],
        &[Unsupported],
    );
}

fn collect_ids<'a>(blocks: &'a [Located<BlockNode>], ids: &mut Vec<&'a str>) {
    for block in blocks {
        match &block.value {
            BlockNode::Heading(heading) => {
                if let Some(id) = &heading.id {
                    ids.push(id.as_str());
                }
            }
            BlockNode::CodeBlock(code) => ids.push(code.id.as_str()),
            BlockNode::Table(table) => ids.push(table.id.as_str()),
            BlockNode::Callout(callout) => {
                ids.push(callout.id.as_str());
                collect_ids(&callout.blocks, ids);
            }
            BlockNode::CodeGroup(group) => {
                ids.extend(
                    group
                        .entries
                        .iter()
                        .map(|entry| entry.value.code.id.as_str()),
                );
            }
            BlockNode::Quote(quote) => collect_ids(&quote.blocks, ids),
            BlockNode::List(list) => {
                for item in &list.items {
                    collect_ids(&item.value.blocks, ids);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn ids_are_unique_document_ordered_and_counted_per_kind() {
    let input = r#"# 記事

本文

- 項目

> 引用

## 第一節

### 小節一

```text
first
```

:::callout
補足

```text
second
```

| 名称 |
| --- |
| 値 |
:::

:::code-group
```text A
third
```
```text B
fourth
```
:::

### 小節二

| 名称 |
| --- |
| 値 |

:::callout{type=warning}
警告
:::

## 第二節

### 次の小節

```text
fifth
```
"#;
    let before = accepted(input);
    let after = accepted(&format(&before));
    for document in [&before, &after] {
        let BlockNode::Heading(title) = &document.blocks()[0].value else {
            panic!("Title was lost");
        };
        assert_eq!(title.level, HeadingLevel::H1);
        assert!(title.id.is_none(), "h1 must not have a reference ID");
        assert!(matches!(
            document.blocks()[1].value,
            BlockNode::Paragraph(_)
        ));
        assert!(matches!(document.blocks()[2].value, BlockNode::List(_)));
        assert!(matches!(document.blocks()[3].value, BlockNode::Quote(_)));
        let mut ids = Vec::new();
        collect_ids(document.blocks(), &mut ids);
        assert_eq!(
            ids,
            [
                "section-01",
                "section-01-01",
                "code-block-001",
                "callout-001",
                "code-block-002",
                "table-001",
                "code-block-003",
                "code-block-004",
                "section-01-02",
                "table-002",
                "callout-002",
                "section-02",
                "section-02-01",
                "code-block-005",
            ]
        );
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    }
}

#[test]
fn orphan_h3_gets_root_section_ids() {
    let input = "### 孤立一\n\n### 孤立二\n\n## 親節\n\n### 子節\n";
    let before = accepted(input);
    let after = accepted(&format(&before));
    for document in [&before, &after] {
        let mut ids = Vec::new();
        collect_ids(document.blocks(), &mut ids);
        assert_eq!(
            ids,
            ["section-01", "section-02", "section-03", "section-03-01"]
        );
    }
}

#[test]
fn folding_markers_are_intent_not_visible_heading_text() {
    let input = "### > 閉じる **強調**\n\n### v 開く\n\n### 通常\n\n### valueは記号でない\n";
    let before = accepted(input);
    let canonical = format(&before);
    let after = accepted(&canonical);
    for document in [&before, &after] {
        let headings: Vec<_> = document
            .blocks()
            .iter()
            .map(|block| {
                let BlockNode::Heading(heading) = &block.value else {
                    panic!("Expected h3, got: {block:?}");
                };
                assert_eq!(heading.level, HeadingLevel::H3);
                (heading.fold, visible_text(&heading.content))
            })
            .collect();
        assert_eq!(
            headings,
            [
                (FoldIntent::InitiallyClosed, "閉じる 強調".into()),
                (FoldIntent::InitiallyOpen, "開く".into()),
                (FoldIntent::None, "通常".into()),
                (FoldIntent::None, "valueは記号でない".into()),
            ]
        );
    }
    assert!(canonical.contains("### > 閉じる **強調**"));
    assert!(canonical.contains("### v 開く"));
    assert!(after.same_meaning(&before));
}

#[test]
fn quoted_attributes_and_boolean_flags_have_one_canonical_form() {
    let single_quotes = "::image{zoom caption='画像の 説明' alt='図の 説明' src='figure.png'}\n\n::video{loop caption='動画の 説明' src=clip.mp4}\n";
    let double_quotes = "::image{src=figure.png alt=\"図の 説明\" caption=\"画像の 説明\" zoom}\n\n::video{src=\"clip.mp4\" caption=\"動画の 説明\" loop}\n";
    let first = accepted(single_quotes);
    let second = accepted(double_quotes);
    let canonical = format(&first);
    assert_eq!(canonical, format(&second));
    assert!(canonical.contains("alt=\"図の 説明\""));
    assert!(canonical.contains("caption=\"画像の 説明\""));
    assert!(!canonical.contains("zoom="));
    assert!(!canonical.contains("loop="));
    assert!(!canonical.contains("'図の 説明'"));
    let after = accepted(&canonical);
    for document in [&first, &second, &after] {
        assert_eq!(document.blocks().len(), 2);
        let BlockNode::Image(image) = &document.blocks()[0].value else {
            panic!("Image attributes were lost");
        };
        assert_eq!(image.src, "figure.png");
        assert_eq!(image.alt.as_deref(), Some("図の 説明"));
        assert_eq!(image.caption.as_deref(), Some("画像の 説明"));
        assert!(image.zoom);
        let BlockNode::Video(video) = &document.blocks()[1].value else {
            panic!("Video attributes were lost");
        };
        assert_eq!(video.src, "clip.mp4");
        assert_eq!(video.caption.as_deref(), Some("動画の 説明"));
        assert!(video.loop_playback);
    }
    assert!(after.same_meaning(&first));
    let omitted = accepted("::image{src=figure.png}\n\n::video{src=clip.mp4}\n");
    let omitted_after = accepted(&format(&omitted));
    for document in [&omitted, &omitted_after] {
        let BlockNode::Image(image) = &document.blocks()[0].value else {
            panic!("Expected image");
        };
        let BlockNode::Video(video) = &document.blocks()[1].value else {
            panic!("Expected video");
        };
        assert!(!image.zoom);
        assert!(!video.loop_playback);
    }
    let error = check_canonical(single_quotes).expect_err("Noncanonical attributes accepted");
    assert_eq!(error.diagnostics()[0].class, DiagnosticClass::NonCanonical);
    check_canonical(&canonical).expect("Canonical attributes rejected");
}

#[test]
fn unknown_attributes_are_rejected_for_the_closed_component_set() {
    use DiagnosticClass::Unsupported;
    assert_rejected_cases(
        &[
            (
                "Callout unknown attribute",
                ":::callout{mystery=value}\n本文\n:::\n",
                Unsupported,
            ),
            (
                "CodeGroup unknown attribute",
                ":::code-group{mystery=value}\n```text label\ncode\n```\n:::\n",
                Unsupported,
            ),
            (
                "SVG unknown attribute",
                ":::svg{mystery=value}\n<svg viewBox=\"0 0 10 10\"></svg>\n:::\n",
                Unsupported,
            ),
            (
                "Image unknown attribute",
                "::image{src=figure.png mystery=value}\n",
                Unsupported,
            ),
            (
                "Video unknown attribute",
                "::video{src=clip.mp4 mystery=value}\n",
                Unsupported,
            ),
            (
                "URL card unknown attribute",
                "::url-card{href=https://example.com/ mystery=value}\n",
                Unsupported,
            ),
            (
                "Tweet card unknown attribute",
                "::tweet-card{href=https://x.com/example/status/1 mystery=value}\n",
                Unsupported,
            ),
        ],
        &[],
    );
}

#[test]
fn duplicate_attributes_never_silently_override_values() {
    use DiagnosticClass::InvalidSyntax;
    assert_rejected_cases(
        &[
            (
                "duplicate value",
                "::image{src=first.png src=second.png}\n",
                InvalidSyntax,
            ),
            (
                "duplicate flag",
                "::image{src=figure.png zoom zoom}\n",
                InvalidSyntax,
            ),
            (
                "duplicate quoted label",
                ":::callout{label='first label' label=\"second label\"}\n本文\n:::\n",
                InvalidSyntax,
            ),
        ],
        &[DiagnosticClass::Semantic],
    );
}
