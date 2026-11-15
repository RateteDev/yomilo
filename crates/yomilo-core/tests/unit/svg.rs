use roxmltree::Document as XmlDocument;
use std::collections::{BTreeSet, HashSet};
use yomilo_core::{
    BlockNode, DiagnosticClass, HeadingLevel, InputState, PreparationInputs, RenderBlockNode,
    RenderSegmentContent, StructureError, StructuredDocument, Svg, check_canonical, format,
    prepare_segment, segment, structure,
};

const ARTICLE_SVG: &str = include_str!("../fixtures/svg/display-catalog.svg");
const ARTICLE_ALT: &str = "記事のMDXから共有の描画へ矢印が伸び、そこからWebとCLIの2つへ矢印が分かれる図。点がMDXから共有の描画へ進み、分かれた後はWebとCLIへ同時に届く";
const ARTICLE_CAPTION: &str = "図:同じMDXが共有の描画を通ってWebとCLIへ届く";

fn svg_source(body: &str) -> String {
    format!(":::svg\n{}\n:::\n", body.trim_end_matches('\n'))
}

fn accepted(input: &str) -> StructuredDocument {
    structure(input).unwrap_or_else(|error| {
        panic!(
            "Expected safe SVG to be accepted.\nInput:\n{input}\nActual: {error:#?}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    })
}

fn round_trip(input: &str) -> (StructuredDocument, StructuredDocument) {
    let before = accepted(input);
    let canonical = format(&before);
    let after = accepted(&canonical);
    assert_eq!(
        format(&after),
        canonical,
        "SVG formatting is not idempotent.\nInput:\n{input}"
    );
    assert!(
        after.same_meaning(&before),
        "SVG round trip changed meaning.\nInput:\n{input}\nCanonical:\n{canonical}\nBefore: {before:#?}\nAfter: {after:#?}"
    );
    check_canonical(&canonical).unwrap_or_else(|error| {
        panic!(
            "Formatted SVG was rejected as noncanonical.\nInput:\n{input}\nCanonical:\n{canonical}\nDiagnostics: {:#?}",
            error.diagnostics()
        )
    });
    (before, after)
}

fn svgs(document: &StructuredDocument) -> Vec<&Svg> {
    document
        .blocks()
        .iter()
        .filter_map(|block| match &block.value {
            BlockNode::Svg(svg) => Some(svg),
            _ => None,
        })
        .collect()
}

fn xml(svg: &Svg) -> XmlDocument<'_> {
    XmlDocument::parse(svg.body.as_str()).unwrap_or_else(|error| {
        panic!(
            "Expected validated SVG to remain XML.\nBody:\n{}\nActual: {error}",
            svg.body.as_str()
        )
    })
}

fn assert_url_target(xml: &XmlDocument<'_>, value: &str, expected_element: &str) {
    let target_id = value
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or_else(|| panic!("Expected a local url(#id) reference, got: {value}"));
    let target = xml
        .descendants()
        .find(|node| node.is_element() && node.attribute("id") == Some(target_id))
        .unwrap_or_else(|| panic!("Local SVG reference was disconnected: {value}"));
    assert_eq!(target.tag_name().name(), expected_element, "{value}");
}

#[test]
fn real_article_svg_accepts_smil_and_preserves_its_meaning() {
    let input = format!(
        "# 実記事SVG\n\n## 共有の描画\n\n:::svg{{zoom alt=\"{ARTICLE_ALT}\" caption=\"{ARTICLE_CAPTION}\"}}\n{ARTICLE_SVG}:::\n"
    );
    let (before, after) = round_trip(&input);
    for document in [&before, &after] {
        let svg_nodes = svgs(document);
        assert_eq!(svg_nodes.len(), 1);
        let svg = svg_nodes[0];
        assert_eq!(svg.alt.as_deref(), Some(ARTICLE_ALT));
        assert_eq!(svg.caption.as_deref(), Some(ARTICLE_CAPTION));
        assert!(svg.zoom);
        let xml = xml(svg);
        for (tag, expected_count) in [
            ("defs", 1),
            ("marker", 1),
            ("path", 4),
            ("rect", 6),
            ("text", 4),
            ("circle", 3),
            ("animate", 5),
            ("animateMotion", 3),
            ("mpath", 3),
        ] {
            assert_eq!(
                xml.descendants()
                    .filter(|node| node.is_element() && node.tag_name().name() == tag)
                    .count(),
                expected_count,
                "Real article SVG lost or added {tag}"
            );
        }
        assert_eq!(
            xml.descendants()
                .filter(|node| node.is_element() && node.tag_name().name() == "text")
                .map(|node| node.text().expect("Article label was lost"))
                .collect::<Vec<_>>(),
            ["記事のMDX", "共有の描画", "Web", "CLI"]
        );
        let colors: BTreeSet<_> = xml
            .descendants()
            .filter(|node| node.is_element())
            .flat_map(|node| [node.attribute("fill"), node.attribute("stroke")])
            .flatten()
            .filter(|value| value.starts_with("var("))
            .collect();
        assert_eq!(
            colors,
            BTreeSet::from([
                "var(--ink-sub)",
                "var(--surface-subtle)",
                "var(--border)",
                "var(--accent)",
            ])
        );
        let animated_paths: Vec<_> = xml
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "mpath")
            .map(|node| {
                let id = node
                    .attribute("href")
                    .and_then(|href| href.strip_prefix('#'))
                    .expect("Motion reference must remain local");
                let path = xml
                    .descendants()
                    .find(|node| node.is_element() && node.attribute("id") == Some(id))
                    .expect("Motion reference lost its path");
                assert_eq!(path.tag_name().name(), "path");
                assert_url_target(
                    &xml,
                    path.attribute("marker-end").expect("Arrow marker was lost"),
                    "marker",
                );
                path.attribute("d").expect("Motion path geometry was lost")
            })
            .collect();
        assert_eq!(
            animated_paths,
            [
                "M128 148H172",
                "M304 148C328 148 324 68 348 68",
                "M304 148C328 148 324 228 348 228",
            ]
        );
        let opacity_values: Vec<_> = xml
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "animate")
            .map(|node| {
                assert_eq!(node.attribute("attributeName"), Some("opacity"));
                assert_eq!(node.attribute("dur"), Some("3s"));
                assert_eq!(node.attribute("repeatCount"), Some("indefinite"));
                node.attribute("values")
                    .expect("Opacity animation was lost")
            })
            .collect();
        assert_eq!(
            opacity_values,
            [
                "0;0;1;0",
                "0;0;1;0",
                "1;1;0;0",
                "0;0;1;1;0;0",
                "0;0;1;1;0;0",
            ]
        );
    }
}

#[test]
fn safe_svg_paint_and_effect_categories_are_accepted_and_retained() {
    let body = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 60">
<defs>
  <linearGradient id="linear"><stop offset="0%" stop-color="var(--accent)"/><stop offset="100%" stop-color="currentColor"/></linearGradient>
  <radialGradient id="radial"><stop offset="0%" stop-color="var(--accent)"/><stop offset="100%" stop-color="currentColor"/></radialGradient>
  <marker id="arrow" markerWidth="3" markerHeight="3" refX="1" refY="1" orient="auto"><path d="M0 0L2 1L0 2z" fill="currentColor"/></marker>
  <clipPath id="clip"><rect width="80" height="40"/></clipPath>
  <filter id="blur"><feGaussianBlur in="SourceGraphic" stdDeviation="1"/></filter>
</defs>
<rect width="100" height="50" fill="url(#linear)" clip-path="url(#clip)" filter="url(#blur)"/>
<circle cx="90" cy="30" r="10" fill="url(#radial)"/>
<path d="M0 55L100 55" stroke="currentColor" marker-end="url(#arrow)"/>
<text x="2" y="20" fill="currentColor">安全な文字</text>
</svg>"#;
    let (before, after) = round_trip(&svg_source(body));
    for document in [&before, &after] {
        let svg_nodes = svgs(document);
        assert_eq!(svg_nodes.len(), 1);
        let xml = xml(svg_nodes[0]);
        for tag in [
            "linearGradient",
            "radialGradient",
            "marker",
            "clipPath",
            "filter",
            "feGaussianBlur",
        ] {
            assert_eq!(
                xml.descendants()
                    .filter(|node| node.is_element() && node.tag_name().name() == tag)
                    .count(),
                1,
                "Safe SVG capability was lost: {tag}"
            );
        }
        let rectangle = xml
            .descendants()
            .find(|node| {
                node.is_element()
                    && node.tag_name().name() == "rect"
                    && node.attribute("width") == Some("100")
            })
            .expect("Painted rectangle was lost");
        for (attribute, target) in [
            ("fill", "linearGradient"),
            ("clip-path", "clipPath"),
            ("filter", "filter"),
        ] {
            assert_url_target(
                &xml,
                rectangle
                    .attribute(attribute)
                    .expect("Paint reference was lost"),
                target,
            );
        }
        let circle = xml
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "circle")
            .expect("Gradient circle was lost");
        assert_url_target(
            &xml,
            circle.attribute("fill").expect("Radial gradient was lost"),
            "radialGradient",
        );
        let text = xml
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "text")
            .expect("SVG text was lost");
        assert_eq!(text.text(), Some("安全な文字"));
        assert_eq!(text.attribute("fill"), Some("currentColor"));
        assert!(xml.descendants().any(|node| {
            node.is_element() && node.attribute("stop-color") == Some("var(--accent)")
        }));
    }
}

#[test]
fn svg_author_ids_survive_round_trip_and_only_prepared_ids_are_scoped() {
    let body = r##"<svg viewBox="0 0 10 10">
<defs>
  <marker id="section-01" markerWidth="3" markerHeight="3" refX="1" refY="1" orient="auto"><path d="M0 0L2 1L0 2z" fill="currentColor"/></marker>
  <path id="shape" d="M1 1L9 9"/>
</defs>
<path d="M0 5L10 5" stroke="currentColor" marker-end="url(#section-01)"/>
<use href="#shape" fill="currentColor"/>
</svg>"##;
    let input = format!("## 節\n\n{}\n{}", svg_source(body), svg_source(body));
    let (before, after) = round_trip(&input);
    for document in [&before, &after] {
        let BlockNode::Heading(heading) = &document.blocks()[0].value else {
            panic!("Heading was lost");
        };
        assert_eq!(heading.level, HeadingLevel::H2);
        let heading_id = heading.id.as_ref().expect("Heading ID was lost").as_str();
        assert_eq!(heading_id, "section-01");
        let svg_nodes = svgs(document);
        assert_eq!(svg_nodes.len(), 2);
        for svg in svg_nodes {
            let xml = xml(svg);
            assert_eq!(
                xml.descendants()
                    .filter(|node| node.is_element())
                    .filter_map(|node| node.attribute("id"))
                    .collect::<Vec<_>>(),
                ["section-01", "shape"],
                "Structure or formatting replaced author SVG IDs.\nInput:\n{input}\nActual body:\n{}",
                svg.body.as_str()
            );
            let shape = xml
                .descendants()
                .find(|node| {
                    node.is_element()
                        && node.tag_name().name() == "path"
                        && node.attribute("d") == Some("M1 1L9 9")
                })
                .expect("Author's referenced shape was lost");
            assert_eq!(shape.attribute("id"), Some("shape"));
            for (tag, attribute, expected) in [
                ("marker", "id", "section-01"),
                ("path", "marker-end", "url(#section-01)"),
                ("use", "href", "#shape"),
            ] {
                assert!(
                    xml.descendants().any(|node| {
                        node.is_element()
                            && node.tag_name().name() == tag
                            && node.attribute(attribute) == Some(expected)
                    }),
                    "Author SVG ID or reference was changed: {tag} {attribute}={expected}\nInput:\n{input}\nActual body:\n{}",
                    svg.body.as_str()
                );
            }
        }

        let source = yomilo_core::format(document);
        let rendered: Vec<_> = segment(&source, None, InputState::Complete)
            .unwrap_or_else(|error| panic!("Valid SVG was rejected.\nInput:\n{input}\n{error:#?}"))
            .segments
            .iter()
            .map(|segment| {
                let prepared = prepare_segment(segment, &PreparationInputs::default())
                    .unwrap_or_else(|error| {
                        panic!(
                            "Preparation rejected valid SVG.\nInput:\n{input}\nActual: {error:#?}"
                        )
                    });
                let RenderSegmentContent::Block(block) = prepared.content else {
                    panic!("Expected a prepared block.\nInput:\n{input}\nActual: {prepared:#?}");
                };
                block
            })
            .collect();
        let RenderBlockNode::SectionHeading(rendered_heading) = &rendered[0].value else {
            panic!("Prepared heading was lost: {rendered:#?}");
        };
        assert_eq!(rendered_heading.id.as_str(), heading_id);
        let mut document_ids = HashSet::from([heading_id.to_owned()]);
        let rendered_svgs: Vec<_> = rendered
            .iter()
            .filter_map(|block| match &block.value {
                RenderBlockNode::Svg(svg) => Some(svg),
                _ => None,
            })
            .collect();
        assert_eq!(rendered_svgs.len(), 2);
        for (svg, prefix) in rendered_svgs
            .into_iter()
            .zip(["yomilo-svg-001", "yomilo-svg-002"])
        {
            let xml = xml(svg);
            for id in xml
                .descendants()
                .filter(|node| node.is_element())
                .filter_map(|node| node.attribute("id"))
            {
                assert!(
                    document_ids.insert(id.to_owned()),
                    "SVG ID collides with another SVG or the heading: {id}\nInput:\n{input}\nActual body:\n{}",
                    svg.body.as_str()
                );
            }
            let marker = xml
                .descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "marker")
                .expect("Scoped marker was lost");
            let marker_id = marker.attribute("id").expect("Scoped marker ID was lost");
            assert_ne!(marker_id, heading_id);
            assert!(marker_id.starts_with(prefix), "{marker_id}");
            let path = xml
                .descendants()
                .find(|node| node.is_element() && node.attribute("d") == Some("M0 5L10 5"))
                .expect("Path using the marker was lost");
            assert_eq!(
                path.attribute("marker-end"),
                Some(format!("url(#{marker_id})").as_str())
            );
            let shape = xml
                .descendants()
                .find(|node| node.is_element() && node.attribute("d") == Some("M1 1L9 9"))
                .expect("Referenced shape was lost");
            let shape_id = shape.attribute("id").expect("Scoped shape ID was lost");
            assert_ne!(shape_id, "shape");
            assert!(shape_id.starts_with(prefix), "{shape_id}");
            let instance = xml
                .descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "use")
                .expect("Local shape instance was lost");
            assert_eq!(
                instance.attribute("href"),
                Some(format!("#{shape_id}").as_str())
            );
        }
        assert_eq!(document_ids.len(), 5);
    }
}

#[test]
fn svg_comments_are_accepted_without_activating_their_contents() {
    let body = r#"<svg viewBox="0 0 20 10">
<!-- source example: <script>alert(1)</script> https://example.com/ -->
<text x="1" y="8" fill="currentColor">コメントを含む図</text>
</svg>"#;
    let (before, after) = round_trip(&svg_source(body));
    for document in [&before, &after] {
        let svg_nodes = svgs(document);
        assert_eq!(svg_nodes.len(), 1);
        let xml = xml(svg_nodes[0]);
        assert!(
            !xml.descendants()
                .any(|node| { node.is_element() && node.tag_name().name() == "script" })
        );
        let text = xml
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "text")
            .expect("Text beside the SVG comment was lost");
        assert_eq!(text.text(), Some("コメントを含む図"));
    }
}

#[test]
fn svg_cdata_is_accepted_as_text_not_markup() {
    let body = r#"<svg viewBox="0 0 20 10"><text x="1" y="8" fill="currentColor"><![CDATA[安全な <文字> & データ]]></text></svg>"#;
    let (before, after) = round_trip(&svg_source(body));
    for document in [&before, &after] {
        let svg_nodes = svgs(document);
        assert_eq!(svg_nodes.len(), 1);
        let xml = xml(svg_nodes[0]);
        let text = xml
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "text")
            .expect("CDATA text was lost");
        assert_eq!(text.text(), Some("安全な <文字> & データ"));
        assert!(!text.children().any(|node| node.is_element()));
    }
}

#[test]
fn unsafe_svg_capability_categories_are_rejected() {
    let cases = [
        (
            "script",
            r#"<svg viewBox="0 0 10 10"><script>alert(1)</script></svg>"#,
        ),
        (
            "event attribute",
            r#"<svg viewBox="0 0 10 10"><rect width="10" height="10" onclick="alert(1)"/></svg>"#,
        ),
        (
            "foreignObject",
            r#"<svg viewBox="0 0 10 10"><foreignObject width="10" height="10"><div xmlns="http://www.w3.org/1999/xhtml">HTML</div></foreignObject></svg>"#,
        ),
        (
            "style element",
            r#"<svg viewBox="0 0 10 10"><style>rect { fill: currentColor; }</style><rect width="10" height="10"/></svg>"#,
        ),
        (
            "style attribute",
            r#"<svg viewBox="0 0 10 10"><rect width="10" height="10" style="fill:currentColor"/></svg>"#,
        ),
        (
            "external URL",
            r#"<svg viewBox="0 0 10 10"><use href="https://example.com/shapes.svg#shape"/></svg>"#,
        ),
        (
            "external font",
            r#"<svg viewBox="0 0 10 10"><defs><font-face><font-face-src><font-face-uri href="https://example.com/font.svg#font"/></font-face-src></font-face></defs></svg>"#,
        ),
        (
            "data reference",
            r#"<svg viewBox="0 0 10 10"><image href="data:image/svg+xml;base64,PHN2Zy8+" width="10" height="10"/></svg>"#,
        ),
        (
            "DTD",
            r#"<!DOCTYPE svg><svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
        ),
        (
            "processing instruction",
            r#"<?yomilo safe?><svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
        ),
        (
            "external entity",
            r#"<!DOCTYPE svg [<!ENTITY external SYSTEM "https://example.com/entity.svg">]><svg viewBox="0 0 10 10"><text>&external;</text></svg>"#,
        ),
        (
            "SMIL external reference",
            r##"<svg viewBox="0 0 10 10"><defs><path id="shape" d="M0 0L10 10"/></defs><use href="#shape"><set attributeName="href" to="https://example.com/other.svg#shape" begin="0s"/></use></svg>"##,
        ),
        (
            "SMIL style mutation",
            r#"<svg viewBox="0 0 10 10"><rect width="10" height="10"><animate attributeName="style" values="fill:currentColor;display:none" dur="1s"/></rect></svg>"#,
        ),
    ];
    let mut failures = Vec::new();
    for (category, body) in cases {
        let input = svg_source(body);
        match structure(&input) {
            Ok(document) => failures.push(format!(
                "Unsafe SVG was accepted.\nCategory: {category}\nInput:\n{input}\nExpected rejection.\nActual: {document:#?}"
            )),
            Err(error) => {
                if !matches!(error, StructureError::Rejected(_))
                    || error.diagnostics().is_empty()
                    || error
                        .diagnostics()
                        .iter()
                        .any(|diagnostic| diagnostic.class == DiagnosticClass::NonCanonical)
                {
                    failures.push(format!(
                        "Expected user-facing safety rejection, not an invariant or formatting error.\nCategory: {category}\nInput:\n{input}\nActual: {error:#?}\nDiagnostics: {:#?}",
                        error.diagnostics()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
