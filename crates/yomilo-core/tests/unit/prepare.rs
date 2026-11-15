use std::collections::BTreeMap;
use yomilo_core::{
    CodeHighlighting, CodeLanguage, FetchState, HeadingLevel, InlineNode, InputState, Located,
    MediaDelivery, MediaKind, PlainCodeReason, PreparationError, PreparationInputs,
    PreparationVersions, RenderBlockNode, RenderCodeBlock, RenderSectionHeading, RenderSegment,
    RenderSegmentContent, SectionFold, Segment, SegmentContent, TweetAuthor, TweetCounts,
    TweetData, TweetDateTime, TweetState, TweetUnavailableReason, UrlCardFetch, prepare_segment,
    segment,
};

#[derive(Debug)]
struct Prepared(Vec<RenderSegment>);

impl Prepared {
    fn blocks(&self) -> Vec<&Located<RenderBlockNode>> {
        self.0
            .iter()
            .map(|segment| match &segment.content {
                RenderSegmentContent::Block(block) => block,
                other => panic!("Expected a prepared block segment: {other:#?}"),
            })
            .collect()
    }
}

fn segments(input: &str) -> Vec<Segment> {
    let batch = segment(input, None, InputState::Complete).unwrap_or_else(|error| {
        panic!("Preparation fixture was rejected by Core.\nInput:\n{input}\nActual: {error:#?}")
    });
    for segment in &batch.segments {
        assert!(
            !matches!(segment.content, SegmentContent::Rejected(_)),
            "Preparation fixture was rejected by Core.\nInput:\n{input}\nActual: {segment:#?}"
        );
    }
    batch.segments
}

fn prepared(input: &str, inputs: &PreparationInputs) -> Prepared {
    Prepared(
        segments(input)
            .iter()
            .map(|segment| {
                prepare_segment(segment, inputs).unwrap_or_else(|error| {
                    panic!("Preparation rejected valid input.\nInput:\n{input}\nActual: {error:#?}")
                })
            })
            .collect(),
    )
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

fn section_headings(document: &Prepared) -> Vec<&RenderSectionHeading> {
    document
        .blocks()
        .into_iter()
        .filter_map(|block| match &block.value {
            RenderBlockNode::SectionHeading(heading) => Some(heading),
            _ => None,
        })
        .collect()
}

fn assert_section(
    section: &RenderSectionHeading,
    id: &str,
    level: HeadingLevel,
    fold: SectionFold,
    title: &str,
) {
    assert_eq!(section.id.as_str(), id);
    assert_eq!(section.level, level);
    assert_eq!(section.fold, fold);
    assert_eq!(visible_text(&section.content), title);
}

#[test]
fn title_and_sections_keep_ids_order_and_three_distinct_fold_states() {
    let input = "# 記事タイトル\n\n## 親節\n\n### 通常\n\n### v 開く\n\n### > 閉じる\n";
    let document = prepared(input, &PreparationInputs::default());
    for segment in &document.0 {
        assert_eq!(
            segment.versions,
            PreparationVersions {
                syntax_version: 0,
                model_version: 0,
                preparation_version: 0,
            }
        );
    }
    assert_eq!(
        document
            .0
            .iter()
            .map(|segment| segment.index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    let RenderBlockNode::Title(title) = &document.blocks()[0].value else {
        panic!("h1 must remain the article title: {document:#?}");
    };
    assert_eq!(visible_text(&title.content), "記事タイトル");
    assert_eq!(title.fold, SectionFold::None);
    let sections = section_headings(&document);
    assert_eq!(sections.len(), 4);
    assert_section(
        sections[0],
        "section-01",
        HeadingLevel::H2,
        SectionFold::None,
        "親節",
    );
    for (child, (id, fold, title, state_name)) in sections[1..].iter().zip([
        ("section-01-01", SectionFold::None, "通常", "none"),
        ("section-01-02", SectionFold::Open, "開く", "open"),
        ("section-01-03", SectionFold::Closed, "閉じる", "closed"),
    ]) {
        assert_section(child, id, HeadingLevel::H3, fold, title);
        assert_eq!(child.fold.as_str(), state_name);
    }
}

#[test]
fn orphan_h3_headings_remain_independent_root_sections() {
    let document = prepared(
        "### 孤立した節\n\n### v 開いた孤立節\n",
        &PreparationInputs::default(),
    );
    let sections = section_headings(&document);
    assert_eq!(sections.len(), 2);
    for (section, (id, fold, title)) in sections.iter().zip([
        ("section-01", SectionFold::None, "孤立した節"),
        ("section-02", SectionFold::Open, "開いた孤立節"),
    ]) {
        assert_section(section, id, HeadingLevel::H3, fold, title);
    }
    assert!(
        !document
            .blocks()
            .iter()
            .any(|block| matches!(block.value, RenderBlockNode::Title(_)))
    );
}

fn single_code(document: &Prepared) -> &RenderCodeBlock {
    assert_eq!(document.blocks().len(), 1);
    let RenderBlockNode::CodeBlock(code) = &document.blocks()[0].value else {
        panic!("Expected a prepared code block: {document:#?}");
    };
    code
}

fn assert_highlighted_tokens(code: &RenderCodeBlock, expected_language: CodeLanguage) {
    let CodeHighlighting::Highlighted { language, tokens } = &code.highlighting else {
        panic!("Known language must be highlighted: {code:#?}");
    };
    assert_eq!(*language, expected_language);
    assert!(
        !tokens.is_empty(),
        "Highlighting lost all tokens: {code:#?}"
    );
    let allowed_roles = [
        "comment",
        "keyword",
        "string",
        "function",
        "type",
        "property",
        "variable",
        "punctuation",
    ];
    for token in tokens {
        assert!(allowed_roles.contains(&token.role.as_str()), "{token:#?}");
        assert!(
            token.start < token.end && token.end <= code.value.len(),
            "{code:#?}"
        );
        assert!(code.value.is_char_boundary(token.start), "{code:#?}");
        assert!(code.value.is_char_boundary(token.end), "{code:#?}");
    }
    assert!(
        tokens.windows(2).all(|pair| pair[0].end <= pair[1].start),
        "Token ranges overlap or are out of order: {code:#?}"
    );
}

#[test]
fn known_code_languages_preserve_utf8_source_and_ordered_role_token_ranges() {
    for (name, language, body) in [
        (
            "rust",
            CodeLanguage::Rust,
            "// 日本語コメント\nfn greet() {\n    let message = \"こんにちは🌱\";\n    println!(\"{}\", message);\n}",
        ),
        (
            "typescript",
            CodeLanguage::TypeScript,
            "// 日本語コメント\nexport function greet(name: string): string {\n  return `こんにちは、${name}🌱`;\n}",
        ),
    ] {
        let input = format!("```{name}\n{body}\n```\n");
        let document = prepared(&input, &PreparationInputs::default());
        let code = single_code(&document);
        assert_eq!(code.language, name);
        assert_eq!(code.value, body);
        assert_highlighted_tokens(code, language);
    }
}

#[test]
fn paragraph_soft_breaks_are_copied_without_affecting_code_tokens() {
    let body = "const greeting = \"こんにちは\";\n// preserved";
    let code_source = format!("```typescript\n{body}\n```\n");
    let source = format!("前\n後\n\n{code_source}");
    let structured = segments(&source);
    let document = prepared(&source, &PreparationInputs::default());
    assert_eq!(document.blocks().len(), 2);
    let RenderBlockNode::Paragraph(paragraph) = &document.blocks()[0].value else {
        panic!("Expected a prepared paragraph: {document:#?}");
    };
    let SegmentContent::Block(Located {
        value: yomilo_core::BlockNode::Paragraph(original),
        ..
    }) = &structured[0].content
    else {
        panic!("Expected a structured paragraph: {structured:#?}");
    };
    assert_eq!(paragraph, original);
    assert_eq!(visible_text(&paragraph.content), "前\n後");
    let RenderBlockNode::CodeBlock(code) = &document.blocks()[1].value else {
        panic!("Expected a prepared code block: {document:#?}");
    };
    assert_eq!(code.value, body);
    assert_highlighted_tokens(code, CodeLanguage::TypeScript);
    let standalone = prepared(&code_source, &PreparationInputs::default());
    assert_eq!(code, single_code(&standalone));
}

#[test]
fn representative_language_aliases_and_new_languages_are_highlighted() {
    for (name, body) in [
        ("py", "def greet():\n    return \"hello\""),
        ("tsx", "export const greeting = <div>Hello</div>;"),
        ("php", "<?php function greet() { return \"hello\"; }"),
        ("swift", "func greet() -> String { return \"hello\" }"),
        ("yml", "greeting: \"hello\""),
    ] {
        let input = format!("```{name}\n{body}\n```\n");
        let document = prepared(&input, &PreparationInputs::default());
        let code = single_code(&document);
        assert_eq!(code.language, name);
        assert_eq!(code.value, body);
        let CodeHighlighting::Highlighted { tokens, .. } = &code.highlighting else {
            panic!("Supported language must be highlighted.\nInput:\n{input}\nActual: {code:#?}");
        };
        assert!(
            !tokens.is_empty(),
            "Supported language produced no highlighting tokens.\nInput:\n{input}\nActual: {code:#?}"
        );
    }
}

#[test]
fn unknown_code_language_succeeds_as_plain_without_losing_source() {
    let body = "未知言語でも本文を保持する🌱";
    for name in ["zzz", "yomilo-test-unknown-language"] {
        let input = format!("```{name}\n{body}\n```\n");
        let document = prepared(&input, &PreparationInputs::default());
        let code = single_code(&document);
        assert_eq!(code.language, name);
        assert_eq!(code.value, body);
        assert_eq!(
            code.highlighting,
            CodeHighlighting::Plain {
                reason: PlainCodeReason::UnknownLanguage
            },
            "Unknown language must remain plain.\nInput:\n{input}\nActual: {code:#?}"
        );
    }
}

#[test]
fn explicit_plain_language_names_preserve_source_and_explicit_reason() {
    for name in ["text", "plain", "plaintext"] {
        let body = "色付けしない本文🌱";
        let input = format!("```{name}\n{body}\n```\n");
        let document = prepared(&input, &PreparationInputs::default());
        let code = single_code(&document);
        assert_eq!(code.language, name);
        assert_eq!(code.value, body);
        assert_eq!(
            code.highlighting,
            CodeHighlighting::Plain {
                reason: PlainCodeReason::ExplicitPlain
            }
        );
    }
}

#[test]
fn code_group_keeps_label_index_code_pairing_and_order() {
    let input = ":::code-group\n```bash npm\nnpm install package\n```\n```text manual\n手動の説明\n```\n:::\n";
    let document = prepared(input, &PreparationInputs::default());
    assert_eq!(document.blocks().len(), 1);
    let RenderBlockNode::CodeGroup(group) = &document.blocks()[0].value else {
        panic!("CodeGroup changed its component kind: {document:#?}");
    };
    assert_eq!(
        group
            .entries
            .iter()
            .map(|entry| (
                entry.value.index,
                entry.value.label.as_str(),
                entry.value.code.language.as_str(),
                entry.value.code.value.as_str(),
            ))
            .collect::<Vec<_>>(),
        [
            (0, "npm", "bash", "npm install package"),
            (1, "manual", "text", "手動の説明"),
        ]
    );
}

#[test]
fn url_card_title_and_icon_states_are_independent_and_keep_component_kind() {
    let expected = [
        (
            "https://example.com/title-only",
            FetchState::Available("取得したタイトル".to_owned()),
            FetchState::Missing,
        ),
        (
            "https://example.com/icon-only",
            FetchState::Missing,
            FetchState::Available(MediaDelivery::Reference {
                reference: "https://example.com/icon.png".to_owned(),
            }),
        ),
        (
            "https://example.com/title-failed",
            FetchState::Failed {
                message: "Title fetch failed".to_owned(),
            },
            FetchState::NotRequested,
        ),
        (
            "https://example.com/icon-failed",
            FetchState::NotRequested,
            FetchState::Failed {
                message: "Icon fetch failed".to_owned(),
            },
        ),
        (
            "https://example.com/not-requested",
            FetchState::NotRequested,
            FetchState::NotRequested,
        ),
    ];
    let input: String = expected
        .iter()
        .map(|(href, _, _)| format!("::url-card{{href=\"{href}\"}}\n\n"))
        .collect();
    let mut inputs = PreparationInputs::default();
    for (href, title, icon) in &expected[..4] {
        inputs.url_cards.insert(
            (*href).to_owned(),
            UrlCardFetch {
                title: title.clone(),
                icon: icon.clone(),
            },
        );
    }
    let document = prepared(&input, &inputs);
    assert_eq!(document.blocks().len(), 5);
    for (block, (href, title, icon)) in document.blocks().iter().zip(&expected) {
        let RenderBlockNode::UrlCard(card) = &block.value else {
            panic!("UrlCard must not become another component after fetch failure: {block:#?}");
        };
        assert_eq!(card.href, *href);
        assert_eq!(&card.title, title);
        assert_eq!(&card.icon, icon);
    }
}

fn tweet(id: &str, created_at: TweetDateTime) -> TweetState {
    TweetState::Available(Box::new(TweetData {
        id: id.to_owned(),
        author: TweetAuthor {
            name: "テスト投稿者".to_owned(),
            handle: "example".to_owned(),
            profile_href: "https://x.com/example".to_owned(),
            avatar: None,
        },
        text: "取得したポスト🌱".to_owned(),
        created_at,
        counts: TweetCounts {
            likes: Some(3),
            ..TweetCounts::default()
        },
        links: Vec::new(),
        media: Vec::new(),
    }))
}

#[test]
fn tweet_card_states_and_source_datetime_values_are_preserved() {
    let expected = [
        (
            "https://x.com/example/status/1",
            tweet(
                "1",
                TweetDateTime::Original("2026-10-12T15:04:05Z".to_owned()),
            ),
        ),
        (
            "https://x.com/example/status/2",
            tweet("2", TweetDateTime::UnixSeconds(1_600_000_000)),
        ),
        (
            "https://x.com/example/status/3",
            tweet("3", TweetDateTime::UnixMilliseconds(1_600_000_000_123)),
        ),
        (
            "https://x.com/example/status/4",
            TweetState::Unavailable {
                reason: TweetUnavailableReason::Missing,
            },
        ),
        (
            "https://x.com/example/status/5",
            TweetState::Unavailable {
                reason: TweetUnavailableReason::Failed {
                    message: "Tweet fetch failed".to_owned(),
                },
            },
        ),
        ("https://x.com/example/status/6", TweetState::NotRequested),
    ];
    let input: String = expected
        .iter()
        .map(|(href, _)| format!("::tweet-card{{href=\"{href}\"}}\n\n"))
        .collect();
    let mut inputs = PreparationInputs::default();
    for (href, state) in &expected[..5] {
        inputs.tweet_cards.insert((*href).to_owned(), state.clone());
    }
    let document = prepared(&input, &inputs);
    assert_eq!(document.blocks().len(), 6);
    for (block, (href, state)) in document.blocks().iter().zip(&expected) {
        let RenderBlockNode::TweetCard(card) = &block.value else {
            panic!("TweetCard must not become another component when unavailable: {block:#?}");
        };
        assert_eq!(card.href, *href);
        assert_eq!(
            &card.state, state,
            "Tweet state or original timestamp was changed"
        );
    }
}

#[test]
fn image_and_video_keep_reference_and_embedded_delivery() {
    let input = "::image{src=reference.png}\n\n::image{src=embedded.svg}\n\n::video{src=reference.mp4}\n\n::video{src=embedded.mp4}\n";
    let image_reference = MediaDelivery::Reference {
        reference: "https://example.com/reference.png".to_owned(),
    };
    let image_embedded = MediaDelivery::Embedded {
        media_type: "image/svg+xml".to_owned(),
        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#.to_vec(),
    };
    let video_reference = MediaDelivery::Reference {
        reference: "https://example.com/reference.mp4".to_owned(),
    };
    let video_embedded = MediaDelivery::Embedded {
        media_type: "video/mp4".to_owned(),
        bytes: vec![0, 0, 0, 24, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm'],
    };
    let inputs = PreparationInputs {
        images: BTreeMap::from([
            ("reference.png".to_owned(), image_reference.clone()),
            ("embedded.svg".to_owned(), image_embedded.clone()),
        ]),
        videos: BTreeMap::from([
            ("reference.mp4".to_owned(), video_reference.clone()),
            ("embedded.mp4".to_owned(), video_embedded.clone()),
        ]),
        ..PreparationInputs::default()
    };
    let document = prepared(input, &inputs);
    assert_eq!(document.blocks().len(), 4);
    for (index, (src, delivery)) in [
        ("reference.png", image_reference),
        ("embedded.svg", image_embedded),
        ("reference.mp4", video_reference),
        ("embedded.mp4", video_embedded),
    ]
    .into_iter()
    .enumerate()
    {
        match &document.blocks()[index].value {
            RenderBlockNode::Image(image) if index < 2 => {
                assert_eq!(image.src, src);
                assert_eq!(image.delivery, delivery);
            }
            RenderBlockNode::Video(video) if index >= 2 => {
                assert_eq!(video.src, src);
                assert_eq!(video.delivery, delivery);
            }
            other => panic!("Media component kind or order changed: {other:#?}"),
        }
    }
}

#[test]
fn image_and_video_without_delivery_input_fail_instead_of_guessing() {
    for (input, expected_kind, expected_reference) in [
        ("::image{src=figure.png}\n", MediaKind::Image, "figure.png"),
        ("::video{src=clip.mp4}\n", MediaKind::Video, "clip.mp4"),
    ] {
        let error = prepare_segment(&segments(input)[0], &PreparationInputs::default())
            .expect_err("Missing media input was accepted");
        let PreparationError::MissingMediaInput {
            kind, reference, ..
        } = &error
        else {
            panic!("Expected MissingMediaInput.\nInput:\n{input}\nActual: {error:#?}");
        };
        assert_eq!(*kind, expected_kind, "Input:\n{input}\nActual: {error:#?}");
        assert_eq!(
            reference, expected_reference,
            "Input:\n{input}\nActual: {error:#?}"
        );
    }
}

#[test]
fn each_preparation_version_mismatch_is_rejected() {
    let input = "# 版の検査\n";
    let segments = segments(input);
    let expected = PreparationVersions {
        syntax_version: 0,
        model_version: 0,
        preparation_version: 0,
    };
    for actual in [
        PreparationVersions {
            syntax_version: 1,
            model_version: 0,
            preparation_version: 0,
        },
        PreparationVersions {
            syntax_version: 0,
            model_version: 1,
            preparation_version: 0,
        },
        PreparationVersions {
            syntax_version: 0,
            model_version: 0,
            preparation_version: 1,
        },
    ] {
        let inputs = PreparationInputs {
            versions: actual,
            ..PreparationInputs::default()
        };
        let error = prepare_segment(&segments[0], &inputs)
            .expect_err("Incompatible preparation versions were accepted");
        assert_eq!(
            error,
            PreparationError::VersionMismatch { expected, actual },
            "Input:\n{input}\nActual versions: {actual:#?}"
        );
    }
}
