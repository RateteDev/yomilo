use super::attributes::{self, Attributes};
use super::{DirectiveKind, RawDirective};
use crate::diagnostic::DiagnosticBag;
use crate::source::{Line, Source};
use crate::{Diagnostic, DiagnosticClass, DiagnosticCode};
use std::ops::Range;

pub(crate) enum SourcePart {
    Markdown(Range<usize>),
    /// ディレクティブが占める行の範囲。検査に通らなかったものは`directive`を持たない。
    Directive {
        range: Range<usize>,
        directive: Option<RawDirective>,
    },
}

#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    length: usize,
    quote_depth: usize,
    list_indent: usize,
}

#[derive(Default)]
struct FenceTracker {
    active: Option<Fence>,
    list_indents: Vec<usize>,
}

fn spaces(text: &str) -> usize {
    text.bytes().take_while(|byte| *byte == b' ').count()
}

fn quote_prefix(mut text: &str) -> (&str, usize) {
    let mut depth = 0;
    loop {
        let indent = spaces(text);
        if indent > 3 || text.as_bytes().get(indent) != Some(&b'>') {
            break;
        }
        text = &text[indent + 1..];
        if text.starts_with(' ') {
            text = &text[1..];
        }
        depth += 1;
    }
    (text, depth)
}

fn list_marker(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let marker_end = if matches!(bytes.first(), Some(b'-' | b'+' | b'*')) {
        1
    } else {
        let digits = bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 || digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
            return None;
        }
        digits + 1
    };
    if bytes.get(marker_end) != Some(&b' ') {
        return None;
    }
    Some(marker_end + spaces(&text[marker_end..]).min(4))
}

impl FenceTracker {
    fn protected(&mut self, line: &str) -> bool {
        let Some(fence) = self.active else {
            return false;
        };
        let (quoted, quote_depth) = quote_prefix(line);
        if !line.trim().is_empty()
            && (quote_depth != fence.quote_depth || spaces(quoted) < fence.list_indent)
        {
            self.active = None;
            return false;
        }
        let content = if quoted.len() >= fence.list_indent {
            &quoted[fence.list_indent..]
        } else {
            ""
        };
        let indent = spaces(content);
        let content = &content[indent..];
        let count = content
            .bytes()
            .take_while(|byte| *byte == fence.marker)
            .count();
        if indent <= 3 && count >= fence.length && content[count..].trim().is_empty() {
            self.active = None;
        }
        true
    }

    fn open(&mut self, line: &str) {
        let (quoted, quote_depth) = quote_prefix(line);
        let mut indent = spaces(quoted);
        let mut content = &quoted[indent..];
        let mut list_indent = 0;
        if let Some(marker) = list_marker(content) {
            let next_indent = indent + marker;
            self.list_indents.retain(|existing| *existing <= indent);
            self.list_indents.push(next_indent);
            list_indent = next_indent;
            content = &content[marker..];
            indent = spaces(content);
            content = &content[indent..];
        } else if let Some(existing) = self
            .list_indents
            .iter()
            .rev()
            .find(|existing| **existing <= indent)
        {
            list_indent = *existing;
            indent -= list_indent;
        } else if !line.trim().is_empty() {
            self.list_indents.clear();
        }
        if indent > 3 {
            return;
        }
        let Some(marker @ (b'`' | b'~')) = content.as_bytes().first().copied() else {
            return;
        };
        let length = content.bytes().take_while(|byte| *byte == marker).count();
        if length >= 3 && (marker != b'`' || !content[length..].contains('`')) {
            self.active = Some(Fence {
                marker,
                length,
                quote_depth,
                list_indent,
            });
        }
    }
}

fn blank(line: Line<'_>) -> bool {
    line.text.trim().is_empty()
}

fn header(
    source: &Source<'_>,
    line: Line<'_>,
    diagnostics: &mut DiagnosticBag,
) -> Result<RawDirective, Diagnostic> {
    let text = line.text.trim_end_matches([' ', '\t']);
    let span = source.span(line.start..line.start + text.len());
    let colons = text.bytes().take_while(|byte| *byte == b':').count();
    if text == ":::" {
        return Err(Diagnostic::new(
            DiagnosticClass::InvalidSyntax,
            DiagnosticCode::UnexpectedDirectiveClose,
            span,
            "Unexpected directive closing marker",
        ));
    }
    let name_end = colons
        + text[colons..]
            .bytes()
            .take_while(|byte| byte.is_ascii_lowercase() || *byte == b'-')
            .count();
    let name = &text[colons..name_end];
    let kind = match name {
        "callout" => DirectiveKind::Callout,
        "code-group" => DirectiveKind::CodeGroup,
        "svg" => DirectiveKind::Svg,
        "image" => DirectiveKind::Image,
        "video" => DirectiveKind::Video,
        "url-card" => DirectiveKind::UrlCard,
        "tweet-card" => DirectiveKind::TweetCard,
        _ => {
            return Err(Diagnostic::new(
                DiagnosticClass::Unsupported,
                DiagnosticCode::UnknownDirective,
                span,
                format!("Unknown directive '{name}'"),
            ));
        }
    };
    if colons != if kind.is_block() { 3 } else { 2 } {
        return Err(Diagnostic::new(
            DiagnosticClass::Unsupported,
            DiagnosticCode::DirectiveForm,
            span,
            "Directive uses the wrong block or single-line form",
        ));
    }
    let remaining = &text[name_end..];
    let attributes = if remaining.is_empty() && kind.is_block() {
        Attributes::empty()
    } else {
        attributes::parse(source, line.start + name_end, remaining, diagnostics)?
    };
    Ok(RawDirective {
        kind,
        header: span,
        attributes,
        body: None,
        span,
    })
}

fn placement(source: &Source<'_>, line: Line<'_>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Unsupported,
        DiagnosticCode::DirectivePlacement,
        source.span(line.start..line.end),
        "Directives must be standalone blocks at column one",
    )
}

fn directive_content(text: &str) -> &str {
    let (quoted, _) = quote_prefix(text);
    let content = quoted.trim_start();
    list_marker(content).map_or(content, |marker| &content[marker..])
}

fn block_end(
    source: &Source<'_>,
    lines: &[Line<'_>],
    start: usize,
    svg: bool,
    diagnostics: &mut DiagnosticBag,
    report_nested: bool,
    misplaced: bool,
) -> Option<usize> {
    let mut tracker = FenceTracker::default();
    let mut depth = 0;
    let mut nested_reported = !report_nested;
    for (index, candidate) in lines.iter().copied().enumerate().skip(start + 1) {
        if !svg && tracker.protected(candidate.text) {
            continue;
        }
        let content = if misplaced {
            directive_content(candidate.text)
        } else {
            candidate.text
        };
        if content.trim_end_matches([' ', '\t']) == ":::" {
            if depth == 0 {
                return Some(index);
            }
            depth -= 1;
        } else if content.trim_start().starts_with("::") {
            if !nested_reported {
                diagnostics.push(Diagnostic::new(
                    DiagnosticClass::Unsupported,
                    DiagnosticCode::NestedDirective,
                    source.span(candidate.start..candidate.end),
                    "Nested or indented directives are not supported",
                ));
                nested_reported = true;
            }
            if content.starts_with(":::") {
                depth += 1;
            }
        }
        if !svg {
            tracker.open(candidate.text);
        }
    }
    None
}

pub(crate) enum Frontmatter {
    Absent,
    /// 開始の行だけがあり、閉じる行がまだ無い。
    Open,
    /// 閉じる行の番号。
    Closed(usize),
}

pub(crate) fn frontmatter(lines: &[Line<'_>]) -> Frontmatter {
    let Some(first) = lines.first() else {
        return Frontmatter::Absent;
    };
    let marker = first.text.trim_end_matches([' ', '\t']);
    if !matches!(marker, "---" | "+++") {
        return Frontmatter::Absent;
    }
    lines
        .iter()
        .skip(1)
        .position(|line| line.text.trim_end_matches([' ', '\t']) == marker)
        .map_or(Frontmatter::Open, |close| Frontmatter::Closed(close + 1))
}

/// `start`は行頭でなければならない。配置の検査が前の行を見るため、行の一覧は原稿の先頭から作る。
pub(crate) fn scan(
    source: &Source<'_>,
    start: usize,
    diagnostics: &mut DiagnosticBag,
) -> Vec<SourcePart> {
    let lines = source.lines(0..source.text.len());
    let mut parts = Vec::new();
    let mut normal_start = start;
    let mut tracker = FenceTracker::default();
    let mut i = lines.partition_point(|line| line.start < start);
    let mut reported_boundary = None;
    if start == 0
        && let Frontmatter::Closed(close) = frontmatter(&lines)
    {
        i = close + 1;
    }
    while i < lines.len() {
        let line = lines[i];
        if tracker.protected(line.text) {
            i += 1;
            continue;
        }
        if line.text.starts_with("::") {
            if normal_start < line.start {
                parts.push(SourcePart::Markdown(normal_start..line.start));
            }
            if i > 0 && !blank(lines[i - 1]) && reported_boundary != Some(i) {
                diagnostics.push(placement(source, line));
            }
            let parsed_header = header(source, line, diagnostics);
            let mut directive = diagnostics.capture(parsed_header);
            let header_end = diagnostics.len();
            let structural_clean = directive.is_some() && (i == 0 || blank(lines[i - 1]));
            let block = directive.as_ref().map_or_else(
                || line.text.starts_with(":::") && line.text.trim_end_matches([' ', '\t']) != ":::",
                |raw| raw.kind.is_block(),
            );
            let end_index = if block {
                let svg = directive
                    .as_ref()
                    .is_some_and(|raw| raw.kind == DirectiveKind::Svg);
                let Some(end) =
                    block_end(source, &lines, i, svg, diagnostics, structural_clean, false)
                else {
                    if structural_clean
                        && diagnostics.len() == header_end
                        && let Some(raw) = &directive
                    {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticClass::InvalidSyntax,
                                DiagnosticCode::UnclosedDirective,
                                raw.header,
                                "Unclosed block directive",
                            )
                            .related(
                                source.span(source.text.len()..source.text.len()),
                                "Reached end of input",
                            ),
                        );
                    }
                    parts.push(SourcePart::Directive {
                        range: line.start..source.text.len(),
                        directive: None,
                    });
                    normal_start = source.text.len();
                    break;
                };
                if let Some(raw) = directive.as_mut() {
                    raw.body = Some(line.end..lines[end].start);
                    raw.span = source.span(
                        line.start
                            ..lines[end].start
                                + lines[end].text.trim_end_matches([' ', '\t']).len(),
                    );
                }
                end
            } else {
                i
            };
            let mut valid = structural_clean && diagnostics.len() == header_end;
            if valid && end_index + 1 < lines.len() && !blank(lines[end_index + 1]) {
                diagnostics.push(placement(source, lines[end_index]));
                reported_boundary = Some(end_index + 1);
                valid = false;
            }
            parts.push(SourcePart::Directive {
                range: line.start..lines[end_index].start + lines[end_index].text.len(),
                directive: directive.filter(|_| valid),
            });
            normal_start = lines[end_index].end;
            i = end_index + 1;
            tracker = FenceTracker::default();
            continue;
        }
        let content = directive_content(line.text);
        if content.starts_with("::") {
            diagnostics.push(placement(source, line));
            let end =
                if content.starts_with(":::") && content.trim_end_matches([' ', '\t']) != ":::" {
                    block_end(source, &lines, i, false, diagnostics, false, true)
                        .unwrap_or(lines.len() - 1)
                } else {
                    i
                };
            diagnostics.suppress(line.start..lines[end].end);
            i = end + 1;
            tracker = FenceTracker::default();
            continue;
        }
        tracker.open(line.text);
        i += 1;
    }
    if normal_start < source.text.len() {
        parts.push(SourcePart::Markdown(normal_start..source.text.len()));
    }
    parts
}
