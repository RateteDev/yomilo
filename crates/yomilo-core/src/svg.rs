mod policy;
mod value;
mod xml;

use crate::diagnostic::DiagnosticBag;
use crate::source::{Source, normalize_line_endings};
use crate::{Diagnostic, DiagnosticClass, DiagnosticCode, SourceSpan, ValidatedSvg};
use policy::Kind;
use roxmltree::{Attribute, Document, Node, ParsingOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const MAX_SVG_NODES: u32 = 10_000;
type Ids<'a, 'input> = BTreeMap<String, Node<'a, 'input>>;

pub(crate) fn validate(
    source: &Source<'_>,
    range: Range<usize>,
    diagnostics: &mut DiagnosticBag,
) -> Result<Option<ValidatedSvg>, Diagnostic> {
    let body = &source.text[range.clone()];
    reject_prolog(source, range.clone())?;
    let document = Document::parse_with_options(
        body,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_SVG_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::InvalidSyntax,
            DiagnosticCode::InvalidSvg,
            source.span(range.clone()),
            format!("Invalid SVG XML: {error}"),
        )
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return Err(unsafe_svg(
            source,
            &range,
            root.range(),
            &format!(
                "SVG root element {:?} is not allowed; expected svg",
                root.tag_name().name()
            ),
        ));
    }
    let before = diagnostics.len();
    let ids = collect_ids(&document);
    let mut elements = BTreeSet::new();
    let mut attributes = BTreeSet::new();
    let mut blocked_end = 0;
    for node in document.descendants() {
        if node.range().start < blocked_end {
            continue;
        }
        if node.is_pi() {
            diagnostics.push(unsafe_svg(
                source,
                &range,
                node.range(),
                "Processing instructions are not allowed",
            ));
            continue;
        }
        if !node.is_element() {
            continue;
        }
        let element = node.tag_name().name();
        if !policy::ELEMENTS.contains(&element)
            || !matches!(node.tag_name().namespace(), None | Some("" | SVG_NAMESPACE))
        {
            let examples = policy::ELEMENTS
                .iter()
                .take(16)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            diagnostics.push(unsafe_svg(source, &range, node.range(), &format!(
                "SVG element {element:?} with namespace {:?} is not allowed. Allowed examples: {examples}", node.tag_name().namespace())));
            blocked_end = node.range().end;
            continue;
        }
        elements.insert(element.to_owned());
        if element == "animateMotion" {
            attributes.insert("transform".to_owned());
        }
        if policy::animation(element) {
            for required in match element {
                "animateTransform" => &["attributeName", "type"][..],
                "animate" | "set" => &["attributeName"][..],
                _ => &[],
            } {
                if node.attribute(*required).is_none() {
                    diagnostics.push(unsafe_svg(source, &range, node.range(), &format!(
                        "Element {element:?} requires attribute {required:?}; safe animation targets include opacity, x, y, fill, and transform")));
                }
            }
        }
        for attribute in node.attributes() {
            let qualified = &body[attribute.range_qname()];
            let Some(kind) = attribute_kind(node, attribute) else {
                attribute_error(
                    source,
                    &range,
                    node,
                    attribute,
                    qualified,
                    "attribute is not in the element allowlist",
                    diagnostics,
                );
                continue;
            };
            attributes.insert(attribute_name(attribute));
            let value = if matches!(kind, Kind::Id) {
                attribute.value()
            } else {
                attribute.value().trim()
            };
            let valid = match kind {
                Kind::AttributeName => animation_kind(node, &ids).map(|_| true),
                Kind::AnimatedValue | Kind::AnimatedValues => {
                    if animation_kind(node, &ids).is_err()
                        || node.tag_name().name() == "animateTransform"
                            && node.attribute("type").is_none()
                    {
                        Ok(true)
                    } else {
                        let valid = if matches!(kind, Kind::AnimatedValues) {
                            value
                                .split(';')
                                .all(|value| animation_value(node, value.trim(), &ids))
                        } else {
                            animation_value(node, value, &ids)
                        };
                        if valid {
                            Ok(true)
                        } else {
                            Err(
                                "animation value must match the safe target attribute grammar; references must remain local #id or url(#id)",
                            )
                        }
                    }
                }
                _ => Ok(value::valid(kind, value)),
            };
            let reason = match valid {
                Ok(true) => None,
                Ok(false) => Some(format!(
                    "value is outside the allowed grammar; expected {}",
                    value::expected(kind)
                )),
                Err(reason) => Some(reason.to_owned()),
            };
            if let Some(reason) = reason {
                attribute_error(
                    source,
                    &range,
                    node,
                    attribute,
                    qualified,
                    &reason,
                    diagnostics,
                );
                continue;
            }
            if matches!(kind, Kind::AttributeName) {
                attributes.insert(
                    animation_attribute_name(node)
                        .expect("validated animation target")
                        .to_owned(),
                );
            }
            if attribute.namespace().is_none()
                && attribute.name() == "id"
                && let Some(previous) = ids.get(value)
                && *previous != node
            {
                let previous = previous.attribute_node("id").expect("collected ID").range();
                diagnostics.push(unsafe_svg(source, &range, attribute.range(), &format!(
                    "Duplicate id={value:?} on element {element:?}; IDs must be unique within one SVG"))
                    .related(source.span(range.start + previous.start..range.start + previous.end), "First SVG ID declaration"));
            }
            let reference_kind = if matches!(kind, Kind::AnimatedValue | Kind::AnimatedValues) {
                animation_kind(node, &ids).ok()
            } else {
                Some(kind)
            };
            let references = reference_kind.map_or_else(Vec::new, |reference_kind| {
                if matches!(kind, Kind::AnimatedValues) {
                    value
                        .split(';')
                        .flat_map(|value| value::references(reference_kind, value.trim()))
                        .collect()
                } else {
                    value::references(reference_kind, value)
                }
            });
            for reference in references {
                let Some(target) = ids.get(reference) else {
                    attribute_error(
                        source,
                        &range,
                        node,
                        attribute,
                        qualified,
                        &format!("reference #{reference} is undefined in this SVG"),
                        diagnostics,
                    );
                    continue;
                };
                if matches!(element, "mpath" | "textPath")
                    && matches!(kind, Kind::Href)
                    && target.tag_name().name() != "path"
                {
                    attribute_error(
                        source,
                        &range,
                        node,
                        attribute,
                        qualified,
                        "reference must identify a path element",
                        diagnostics,
                    );
                }
                if matches!(kind, Kind::Timing)
                    && value::syncbase(value, reference)
                    && !policy::animation(target.tag_name().name())
                {
                    attribute_error(
                        source,
                        &range,
                        node,
                        attribute,
                        qualified,
                        "syncbase reference must identify an animation element",
                        diagnostics,
                    );
                }
                if policy::animation(element)
                    && matches!(kind, Kind::Href)
                    && (policy::animation(target.tag_name().name())
                        || target.tag_name().name() == "mpath")
                {
                    attribute_error(
                        source,
                        &range,
                        node,
                        attribute,
                        qualified,
                        "animations cannot modify other animation elements",
                        diagnostics,
                    );
                }
            }
        }
        if element == "animateMotion"
            && node.attribute("href").is_none()
            && node.attribute((XLINK_NAMESPACE, "href")).is_none()
            && animation_kind(node, &ids).is_err()
        {
            diagnostics.push(unsafe_svg(source, &range, node.range(),
                "animateMotion cannot modify its animation parent; target must be a drawing element"));
        }
    }
    Ok((diagnostics.len() == before).then(|| {
        let normalized = xml::normalize(body, &document);
        attributes.extend(normalized.namespaces);
        ValidatedSvg {
            source: normalize_line_endings(&normalized.source)
                .trim_matches([' ', '\t', '\n'])
                .to_owned(),
            elements: elements.into_iter().collect(),
            attributes: attributes.into_iter().collect(),
        }
    }))
}

fn collect_ids<'a, 'input>(document: &'a Document<'input>) -> Ids<'a, 'input> {
    let mut ids = Ids::new();
    for node in document.descendants().filter(Node::is_element) {
        if let Some(attribute) = node.attribute_node("id")
            && value::id(attribute.value())
        {
            ids.entry(attribute.value().to_owned()).or_insert(node);
        }
    }
    ids
}

fn attribute_kind(node: Node<'_, '_>, attribute: Attribute<'_, '_>) -> Option<Kind> {
    match (attribute.namespace(), attribute.name()) {
        (None, _) => policy::kind(node.tag_name().name(), attribute.name()),
        (Some(XLINK_NAMESPACE), "href") => policy::kind(node.tag_name().name(), "href"),
        (Some(XML_NAMESPACE), "space" | "lang") => Some(Kind::Text),
        _ => None,
    }
}

fn attribute_name(attribute: Attribute<'_, '_>) -> String {
    match attribute.namespace() {
        Some(XLINK_NAMESPACE) => format!("xlink:{}", attribute.name()),
        Some(XML_NAMESPACE) => format!("xml:{}", attribute.name()),
        _ => attribute.name().to_owned(),
    }
}

fn animation_attribute_name<'a, 'input>(node: Node<'a, 'input>) -> Option<&'a str> {
    let name = node.attribute("attributeName")?.trim();
    if let Some((prefix, "href")) = name.split_once(':')
        && node.lookup_namespace_uri(Some(prefix)) == Some(XLINK_NAMESPACE)
    {
        Some("xlink:href")
    } else {
        Some(name)
    }
}

fn animation_kind(node: Node<'_, '_>, ids: &Ids<'_, '_>) -> Result<Kind, &'static str> {
    let target = if let Some(href) = node
        .attribute("href")
        .or_else(|| node.attribute((XLINK_NAMESPACE, "href")))
    {
        value::fragment(href.trim())
            .and_then(|id| ids.get(id))
            .copied()
            .ok_or("animation target must exist within this SVG")?
    } else {
        node.parent_element()
            .ok_or("animation requires a target element")?
    };
    if policy::animation(target.tag_name().name()) || target.tag_name().name() == "mpath" {
        return Err("animations cannot modify other animation elements");
    }
    if node.tag_name().name() == "animateMotion" {
        return Ok(Kind::Points);
    }
    let name = animation_attribute_name(node).ok_or("missing animation attributeName")?;
    if !policy::ANIMATABLE.contains(&name) {
        return Err(
            "attributeName is not a safe animation target; allowed examples: opacity, x, y, fill, stroke, transform",
        );
    }
    if node.tag_name().name() == "animateTransform"
        && !matches!(name, "transform" | "gradientTransform" | "patternTransform")
    {
        return Err(
            "animateTransform must target transform, gradientTransform, or patternTransform",
        );
    }
    let name = if name == "xlink:href" {
        let original = node
            .attribute("attributeName")
            .expect("animation target")
            .trim();
        let prefix = original
            .split_once(':')
            .expect("qualified animation target")
            .0;
        if node.lookup_namespace_uri(Some(prefix)) != Some(XLINK_NAMESPACE) {
            return Err("xlink:href animation requires the standard xlink namespace binding");
        }
        "href"
    } else {
        name
    };
    policy::kind(target.tag_name().name(), name)
        .ok_or("attributeName is not allowed on the animation target element")
}

fn animation_value(node: Node<'_, '_>, value: &str, ids: &Ids<'_, '_>) -> bool {
    if node.tag_name().name() == "animateTransform" {
        return node
            .attribute("type")
            .is_some_and(|kind| value::transform_arguments(kind.trim(), value));
    }
    if node.tag_name().name() == "animateMotion" {
        return value::numbers(value).is_some_and(|values| values.len() == 2);
    }
    animation_kind(node, ids).is_ok_and(|kind| value::valid(kind, value))
}

fn attribute_error(
    source: &Source<'_>,
    range: &Range<usize>,
    node: Node<'_, '_>,
    attribute: Attribute<'_, '_>,
    qualified: &str,
    reason: &str,
    diagnostics: &mut DiagnosticBag,
) {
    let examples = if policy::animation(node.tag_name().name()) {
        "attributeName, values, from, to, dur, begin, end, repeatCount"
    } else {
        "id, fill, stroke, opacity, transform, font-size; geometry and filter attributes are element-specific"
    };
    diagnostics.push(unsafe_svg(source, range, attribute.range(), &format!(
        "SVG element {:?}, attribute {qualified:?}, value {:?}: {reason}. Allowed examples: {examples}", node.tag_name().name(), attribute.value())));
}

fn unsafe_svg(
    source: &Source<'_>,
    body: &Range<usize>,
    local: Range<usize>,
    message: &str,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        DiagnosticCode::UnsafeSvg,
        source.span(body.start + local.start..body.start + local.end),
        message,
    )
}

pub(crate) fn namespace(
    svg: &mut ValidatedSvg,
    prefix: &str,
    span: SourceSpan,
) -> Result<(), Diagnostic> {
    let document = Document::parse(&svg.source).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::ParserFailure,
            span,
            format!("Validated SVG could not be reparsed: {error}"),
        )
    })?;
    let mut ids = BTreeMap::new();
    let targets = collect_ids(&document);
    // 入力IDの出現順で、描画用IDを決定的に割り当てる。
    for node in document.descendants().filter(Node::is_element) {
        if let Some(id) = node.attribute("id") {
            let index = ids.len() + 1;
            ids.insert(id, format!("{prefix}-id-{index:03}"));
        }
    }
    if ids.is_empty() {
        return Ok(());
    }
    let mut edits = Vec::new();
    for node in document.descendants().filter(Node::is_element) {
        for attribute in node.attributes() {
            let kind = attribute_kind(node, attribute).expect("validated attribute");
            let value = attribute.value().trim();
            let rewritten = if matches!(kind, Kind::AnimatedValue | Kind::AnimatedValues) {
                let target_kind =
                    animation_kind(node, &targets).expect("validated animation target");
                if matches!(kind, Kind::AnimatedValues) {
                    let mut changed = false;
                    let tokens: Vec<_> = value
                        .split(';')
                        .map(|token| {
                            rewrite_value(target_kind, token.trim(), &ids).map_or_else(
                                || token.to_owned(),
                                |rewritten| {
                                    changed = true;
                                    rewritten
                                },
                            )
                        })
                        .collect();
                    changed.then(|| tokens.join(";"))
                } else {
                    rewrite_value(target_kind, value, &ids)
                }
            } else {
                rewrite_value(kind, value, &ids)
            };
            if let Some(rewritten) = rewritten {
                edits.push((attribute.range_value(), escape_attribute(&rewritten)));
            }
        }
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut output = String::new();
    let mut end = 0;
    for (range, replacement) in edits {
        output.push_str(&svg.source[end..range.start]);
        output.push_str(&replacement);
        end = range.end;
    }
    output.push_str(&svg.source[end..]);
    svg.source = output;
    Ok(())
}

fn rewrite_value(kind: Kind, value: &str, ids: &BTreeMap<&str, String>) -> Option<String> {
    match kind {
        Kind::Id => Some(ids[value].clone()),
        Kind::Href => Some(format!(
            "#{}",
            ids[value::fragment(value).expect("validated fragment")]
        )),
        Kind::Url | Kind::Paint => {
            value::url_fragment(value).map(|id| format!("url(#{})", ids[id]))
        }
        Kind::IdList => Some(
            value
                .split_ascii_whitespace()
                .map(|id| ids[id].as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        Kind::Timing => Some(
            value
                .split(';')
                .map(|token| {
                    let token = token.trim();
                    value::timing_reference(token).map_or_else(
                        || token.to_owned(),
                        |(id, suffix)| format!("{}{suffix}", ids[id]),
                    )
                })
                .collect::<Vec<_>>()
                .join(";"),
        ),
        _ => None,
    }
}

fn escape_attribute(value: &str) -> String {
    value.chars().fold(String::new(), |mut output, ch| {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
        output
    })
}

fn reject_prolog(source: &Source<'_>, range: Range<usize>) -> Result<(), Diagnostic> {
    // XML宣言と空DTDはroxmltreeの文書木に残らないため、字句として拒否する。
    let text = &source.text[range.clone()];
    let mut i = 0;
    while i < text.len() {
        let remaining = &text[i..];
        if let Some(comment) = remaining.strip_prefix("<!--") {
            if let Some(end) = comment.find("-->") {
                i += 4 + end + 3;
            } else {
                break;
            }
        } else if remaining.starts_with("<![CDATA[") {
            if let Some(end) = remaining.find("]]>") {
                i += end + 3;
            } else {
                break;
            }
        } else if remaining.starts_with("<?") || remaining.starts_with("<!DOCTYPE") {
            return Err(unsafe_svg(
                source,
                &range,
                i..i + 2,
                "DTD and processing instructions are not allowed",
            ));
        } else if remaining.starts_with('<') {
            let mut quote = None;
            i += 1;
            while i < text.len() {
                let ch = text[i..].chars().next().expect("valid UTF-8 boundary");
                i += ch.len_utf8();
                match (quote, ch) {
                    (Some(delimiter), ch) if ch == delimiter => quote = None,
                    (None, '\'' | '"') => quote = Some(ch),
                    (None, '>') => break,
                    _ => {}
                }
            }
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    Ok(())
}
