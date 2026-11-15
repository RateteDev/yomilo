mod attributes;
pub(crate) mod scan;

use self::attributes::Attributes;
use crate::diagnostic::DiagnosticBag;
use crate::markdown::{self, Context};
use crate::source::Source;
use crate::{
    BlockNode, Callout, CalloutType, CodeGroup, CodeGroupEntry, Diagnostic, DiagnosticClass,
    DiagnosticCode, Image, Located, Ratio, ReferenceId, SourceSpan, Svg, TweetCard, UrlCard, Video,
};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DirectiveKind {
    Callout,
    CodeGroup,
    Svg,
    Image,
    Video,
    UrlCard,
    TweetCard,
}

impl DirectiveKind {
    fn is_block(self) -> bool {
        matches!(self, Self::Callout | Self::CodeGroup | Self::Svg)
    }
}

pub(crate) struct RawDirective {
    kind: DirectiveKind,
    header: SourceSpan,
    attributes: Attributes,
    body: Option<Range<usize>>,
    span: SourceSpan,
}

fn ratio(attributes: &Attributes, header: SourceSpan) -> Result<Option<Ratio>, Diagnostic> {
    let Some(value) = attributes.string("ratio")? else {
        return Ok(None);
    };
    let invalid = || {
        Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::InvalidAttributeValue,
            attributes.value_span("ratio", header),
            "Ratio must contain two positive integers separated by '/'",
        )
    };
    let (width, height) = value.split_once('/').ok_or_else(invalid)?;
    if !width.bytes().all(|b| b.is_ascii_digit()) || !height.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let width: u32 = width.parse().map_err(|_| invalid())?;
    let height: u32 = height.parse().map_err(|_| invalid())?;
    if width == 0 || height == 0 {
        return Err(invalid());
    }
    let (mut a, mut b) = (width, height);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Ok(Some(Ratio {
        width: width / a,
        height: height / a,
    }))
}

pub(crate) fn structure(
    source: &Source<'_>,
    raw: RawDirective,
    diagnostics: &mut DiagnosticBag,
) -> Result<Option<Located<BlockNode>>, Diagnostic> {
    let attributes = raw.attributes;
    let header = raw.header;
    let value = match raw.kind {
        DirectiveKind::Callout => {
            attributes.check_names(&["type", "label"], diagnostics);
            let kind = diagnostics.capture(callout_type(&attributes, header));
            let label = diagnostics.capture(attributes.string("label"));
            let parsed = markdown::parse(
                source,
                raw.body.expect("block body was scanned"),
                Context::callout(),
                diagnostics,
            )?;
            kind.zip(label).map(|(kind, label)| {
                BlockNode::Callout(Callout {
                    id: ReferenceId(String::new()),
                    kind,
                    label: label.unwrap_or_else(|| kind.default_label().to_owned()),
                    blocks: parsed.blocks,
                })
            })
        }
        DirectiveKind::CodeGroup => {
            attributes.check_names(&[], diagnostics);
            let (nodes, mapper) =
                markdown::tree(source, raw.body.expect("block body was scanned"), false)?;
            let mut entries = Vec::new();
            let mut labels = BTreeMap::new();
            if nodes.is_empty() {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    DiagnosticCode::InvalidCodeGroup,
                    header,
                    "A code group requires at least one code block",
                ));
            }
            for node in nodes {
                if let Some(entry) = diagnostics.capture(group_entry(node, &mapper, &mut labels)) {
                    entries.push(entry);
                }
            }
            Some(BlockNode::CodeGroup(CodeGroup { entries }))
        }
        DirectiveKind::Svg => {
            attributes.check_names(&["alt", "caption", "zoom"], diagnostics);
            let body = crate::svg::validate(
                source,
                raw.body.expect("block body was scanned"),
                diagnostics,
            );
            let body = diagnostics.capture(body).flatten();
            let alt = diagnostics.capture(attributes.string("alt"));
            let caption = diagnostics.capture(attributes.string("caption"));
            let zoom = diagnostics.capture(attributes.boolean("zoom"));
            match (body, alt, caption, zoom) {
                (Some(body), Some(alt), Some(caption), Some(zoom)) => Some(BlockNode::Svg(Svg {
                    id: ReferenceId(String::new()),
                    body,
                    alt,
                    caption,
                    zoom,
                })),
                _ => None,
            }
        }
        DirectiveKind::Image => {
            attributes.check_names(&["src", "alt", "ratio", "caption", "zoom"], diagnostics);
            let src = diagnostics.capture(reference_attribute(&attributes, "src", header, false));
            let alt = diagnostics.capture(attributes.string("alt"));
            let ratio = diagnostics.capture(ratio(&attributes, header));
            let caption = diagnostics.capture(attributes.string("caption"));
            let zoom = diagnostics.capture(attributes.boolean("zoom"));
            match (src, alt, ratio, caption, zoom) {
                (Some(src), Some(alt), Some(ratio), Some(caption), Some(zoom)) => {
                    Some(BlockNode::Image(Image {
                        src,
                        alt,
                        ratio,
                        caption,
                        zoom,
                    }))
                }
                _ => None,
            }
        }
        DirectiveKind::Video => {
            attributes.check_names(&["src", "ratio", "caption", "loop"], diagnostics);
            let src = diagnostics.capture(reference_attribute(&attributes, "src", header, false));
            let ratio = diagnostics.capture(ratio(&attributes, header));
            let caption = diagnostics.capture(attributes.string("caption"));
            let loop_playback = diagnostics.capture(attributes.boolean("loop"));
            match (src, ratio, caption, loop_playback) {
                (Some(src), Some(ratio), Some(caption), Some(loop_playback)) => {
                    Some(BlockNode::Video(Video {
                        src,
                        ratio,
                        caption,
                        loop_playback,
                    }))
                }
                _ => None,
            }
        }
        DirectiveKind::UrlCard | DirectiveKind::TweetCard => {
            attributes.check_names(&["href"], diagnostics);
            diagnostics
                .capture(reference_attribute(&attributes, "href", header, true))
                .map(|href| {
                    if raw.kind == DirectiveKind::UrlCard {
                        BlockNode::UrlCard(UrlCard { href })
                    } else {
                        BlockNode::TweetCard(TweetCard { href })
                    }
                })
        }
    };
    Ok(value.map(|value| Located {
        value,
        span: raw.span,
    }))
}

fn callout_type(attributes: &Attributes, header: SourceSpan) -> Result<CalloutType, Diagnostic> {
    match attributes.string("type")?.as_deref().unwrap_or("note") {
        "note" => Ok(CalloutType::Note),
        "warning" => Ok(CalloutType::Warning),
        "caution" => Ok(CalloutType::Caution),
        _ => Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::InvalidAttributeValue,
            attributes.value_span("type", header),
            "Callout type must be note, warning, or caution",
        )),
    }
}

fn reference_attribute(
    attributes: &Attributes,
    name: &str,
    header: SourceSpan,
    absolute: bool,
) -> Result<String, Diagnostic> {
    let value = attributes.required(name, header)?;
    crate::validate::reference(&value, attributes.value_span(name, header), false, absolute)?;
    Ok(value)
}

fn group_entry(
    node: ::markdown::mdast::Node,
    mapper: &markdown::Mapper<'_, '_>,
    labels: &mut BTreeMap<String, SourceSpan>,
) -> Result<Located<CodeGroupEntry>, Diagnostic> {
    let span = mapper.span(&node)?;
    let ::markdown::mdast::Node::Code(code) = node else {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::InvalidCodeGroup,
            span,
            "Code groups may contain only fenced code blocks",
        ));
    };
    let label = code
        .meta
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::MissingCodeGroupLabel,
                span,
                "Every code group entry requires a non-empty label",
            )
        })?
        .to_owned();
    if label.chars().any(char::is_control) {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::InvalidCodeGroup,
            span,
            "Code group labels cannot contain control characters",
        ));
    }
    if let Some(previous) = labels.get(&label) {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::DuplicateCodeGroupLabel,
            span,
            "Code group labels must be unique",
        )
        .related(*previous, "First label"));
    }
    labels.insert(label.clone(), span);
    Ok(Located {
        value: CodeGroupEntry {
            label,
            code: mapper.code(code, span, true, 0)?,
        },
        span,
    })
}
