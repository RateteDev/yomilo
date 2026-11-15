use super::{
    SVG_NAMESPACE, XLINK_NAMESPACE, XML_NAMESPACE, animation_attribute_name, attribute_name,
    escape_attribute,
};
use roxmltree::{Document, Node};
use std::{collections::BTreeSet, ops::Range};

pub(super) struct Normalized {
    pub source: String,
    pub namespaces: BTreeSet<String>,
}

pub(super) fn normalize(body: &str, document: &Document<'_>) -> Normalized {
    let root = document.root_element();
    let mut edits = text_edits(body);
    let mut svg_prefix = false;
    let mut xlink = false;
    for node in document.descendants().filter(Node::is_element) {
        let opening = name_range(body, node.range().start + 1);
        if body[opening.clone()].contains(':') {
            svg_prefix = true;
            edits.push((opening, node.tag_name().name().to_owned()));
            let raw = &body[node.range()];
            if !raw.ends_with("/>") {
                let closing = node.range().start + raw.rfind("</").expect("XML closing tag") + 2;
                edits.push((name_range(body, closing), node.tag_name().name().to_owned()));
            }
        }
        for attribute in node.attributes() {
            let name = attribute_name(attribute);
            if body[attribute.range_qname()] != name {
                edits.push((attribute.range_qname(), name));
            }
            xlink |= attribute.namespace() == Some(XLINK_NAMESPACE);
            if attribute.namespace().is_none() && attribute.name() == "attributeName" {
                let name = animation_attribute_name(node).expect("validated animation target");
                xlink |= name == "xlink:href";
                if name != attribute.value() {
                    edits.push((attribute.range_value(), escape_attribute(name)));
                }
            }
        }
    }
    let add_svg = svg_prefix && root.lookup_namespace_uri(None) != Some(SVG_NAMESPACE);
    let add_xlink = xlink && root.lookup_namespace_uri(Some("xlink")) != Some(XLINK_NAMESPACE);
    let mut namespaces = BTreeSet::new();
    for node in document.descendants().filter(Node::is_element) {
        namespace_edits(
            body,
            node,
            node == root && add_svg,
            &mut edits,
            &mut namespaces,
        );
    }
    let mut declarations = String::new();
    if add_svg {
        declarations.push_str(&format!(" xmlns=\"{SVG_NAMESPACE}\""));
        namespaces.insert("xmlns".to_owned());
    }
    if add_xlink {
        declarations.push_str(&format!(" xmlns:xlink=\"{XLINK_NAMESPACE}\""));
        namespaces.insert("xmlns:xlink".to_owned());
    }
    if !declarations.is_empty() {
        let end = name_range(body, root.range().start + 1).end;
        edits.push((end..end, declarations));
    }
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let mut source = String::new();
    let mut end = 0;
    for (range, replacement) in edits {
        source.push_str(&body[end..range.start]);
        source.push_str(&replacement);
        end = range.end;
    }
    source.push_str(&body[end..]);
    Normalized { source, namespaces }
}

fn name_range(body: &str, start: usize) -> Range<usize> {
    let end = body[start..]
        .find(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '=' | '/' | '>'))
        .expect("XML name delimiter");
    start..start + end
}

fn namespace_edits(
    body: &str,
    node: Node<'_, '_>,
    replace_default: bool,
    edits: &mut Vec<(Range<usize>, String)>,
    namespaces: &mut BTreeSet<String>,
) {
    let mut i = name_range(body, node.range().start + 1).end;
    loop {
        let start = i;
        while body.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if matches!(body.as_bytes().get(i), Some(b'/' | b'>')) {
            break;
        }
        let name = name_range(body, i);
        i = name.end;
        while body.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        i += 1;
        while body.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let quote = char::from(body.as_bytes()[i]);
        i += 1;
        i += body[i..].find(quote).expect("XML attribute closing quote") + 1;
        let qualified = &body[name];
        let keep = match qualified {
            "xmlns" => {
                !replace_default
                    && matches!(
                        node.lookup_namespace_uri(None),
                        None | Some("" | SVG_NAMESPACE)
                    )
            }
            "xmlns:xlink" => node.lookup_namespace_uri(Some("xlink")) == Some(XLINK_NAMESPACE),
            "xmlns:xml" => node.lookup_namespace_uri(Some("xml")) == Some(XML_NAMESPACE),
            name if name.starts_with("xmlns:") => false,
            _ => continue,
        };
        if keep {
            namespaces.insert(qualified.to_owned());
        } else {
            edits.push((start..i, String::new()));
        }
    }
}

fn text_edits(body: &str) -> Vec<(Range<usize>, String)> {
    let mut edits = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let remaining = &body[i..];
        if let Some(comment) = remaining.strip_prefix("<!--") {
            let end = i + 4 + comment.find("-->").expect("XML comment terminator") + 3;
            edits.push((i..end, String::new()));
            i = end;
        } else if remaining.starts_with("<![CDATA[") {
            let content_end = i + remaining.find("]]>").expect("XML CDATA terminator");
            // HTML側でCDATAを別の構文として解釈しても、文字データから要素を生成させない。
            edits.push((i..content_end + 3, escape_text(&body[i + 9..content_end])));
            i = content_end + 3;
        } else if remaining.starts_with('<') {
            let mut quote = None;
            i += 1;
            while i < body.len() {
                let ch = body[i..].chars().next().expect("UTF-8 boundary");
                i += ch.len_utf8();
                match (quote, ch) {
                    (Some(delimiter), ch) if ch == delimiter => quote = None,
                    (None, '\'' | '"') => quote = Some(ch),
                    (None, '>') => break,
                    _ => {}
                }
            }
        } else if remaining.starts_with('>') {
            // コメント除去によって通常テキストにCDATA終端が生じることを防ぐ。
            edits.push((i..i + 1, "&gt;".to_owned()));
            i += 1;
        } else {
            i += body[i..].chars().next().expect("UTF-8 boundary").len_utf8();
        }
    }
    edits
}

fn escape_text(value: &str) -> String {
    value.chars().fold(String::new(), |mut output, ch| {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(ch),
        }
        output
    })
}
