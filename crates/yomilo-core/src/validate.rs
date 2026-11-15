use crate::diagnostic::DiagnosticBag;
use crate::source::Source;
use crate::{Diagnostic, DiagnosticClass, DiagnosticCode, SourceSpan};

pub(crate) fn reference(
    value: &str,
    span: SourceSpan,
    mailto: bool,
    absolute: bool,
) -> Result<(), Diagnostic> {
    let unsafe_reference = || {
        Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::UnsafeReference,
            span,
            "Reference must use a safe, unambiguous URI scheme",
        )
    };
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
        || value.starts_with("//")
    {
        return Err(unsafe_reference());
    }
    let boundary = value.find(['/', '?', '#']).unwrap_or(value.len());
    if let Some(colon) = value.find(':').filter(|colon| *colon < boundary) {
        let scheme = &value[..colon];
        if !scheme.starts_with(|ch: char| ch.is_ascii_alphabetic())
            || !scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        {
            return Err(unsafe_reference());
        }
        let scheme = scheme.to_ascii_lowercase();
        if matches!(scheme.as_str(), "http" | "https") {
            let authority = value[colon + 1..]
                .strip_prefix("//")
                .ok_or_else(unsafe_reference)?;
            if authority
                .split(['/', '?', '#'])
                .next()
                .is_none_or(str::is_empty)
            {
                return Err(unsafe_reference());
            }
        } else if !(mailto && !absolute && scheme == "mailto" && colon + 1 < value.len()) {
            return Err(unsafe_reference());
        }
    } else if absolute {
        return Err(unsafe_reference());
    }
    Ok(())
}

pub(crate) fn text_syntax(
    source: &Source<'_>,
    span: SourceSpan,
    inside_link: bool,
    diagnostics: &mut DiagnosticBag,
) {
    let raw = &source.text[span.byte_range()];
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            i += 2;
            continue;
        }
        let remainder = &raw[i..];
        let message = if remainder.starts_with("~~") {
            Some("Strikethrough syntax is not supported")
        } else if remainder.starts_with('<')
            && remainder.as_bytes().get(1).is_some_and(|byte| {
                byte.is_ascii_alphabetic() || matches!(byte, b'/' | b'!' | b'?' | b'>')
            })
        {
            Some("HTML and JSX syntax is not supported")
        } else if remainder.starts_with("![") {
            Some("Markdown images are not supported")
        } else if remainder.starts_with("[^") {
            Some("Footnote syntax is not supported")
        } else if remainder.starts_with("::")
            && (i == 0 || bytes[i - 1].is_ascii_whitespace())
            && remainder
                .as_bytes()
                .get(2)
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b':')
        {
            Some("Directive syntax must be a standalone block")
        } else if !inside_link
            && (remainder.starts_with("http://")
                || remainder.starts_with("https://")
                || remainder.starts_with("www."))
        {
            Some("Bare URLs are not supported")
        } else {
            None
        };
        if let Some(message) = message {
            diagnostics.push(Diagnostic::new(
                DiagnosticClass::Unsupported,
                DiagnosticCode::UnsupportedSyntax,
                source.span(
                    span.start.byte_offset + i
                        ..span.start.byte_offset
                            + i
                            + remainder.chars().next().map_or(0, char::len_utf8),
                ),
                message,
            ));
            let Some(end) = rejected_token_end(raw, i) else {
                return;
            };
            i = end;
            continue;
        }
        if bytes[i] == b'`' || remainder.starts_with("**") {
            diagnostics.push(Diagnostic::new(
                DiagnosticClass::InvalidSyntax,
                DiagnosticCode::InvalidInlineSyntax,
                source.span(span.start.byte_offset + i..span.start.byte_offset + i + 1),
                "Unmatched inline formatting delimiter",
            ));
            return;
        }
        if bytes[i] == b'['
            && let Some(close) = unescaped_close(raw, i + 1, b']')
        {
            if matches!(bytes.get(close + 1), Some(b'[')) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticClass::Unsupported,
                    DiagnosticCode::UnsupportedSyntax,
                    source.span(span.start.byte_offset + i..span.start.byte_offset + close + 2),
                    "Reference links are not supported",
                ));
                let Some(end) = unescaped_close(raw, close + 2, b']') else {
                    return;
                };
                i = end + 1;
                continue;
            }
            if bytes.get(close + 1) == Some(&b'(') {
                diagnostics.push(Diagnostic::new(
                    DiagnosticClass::InvalidSyntax,
                    DiagnosticCode::InvalidInlineSyntax,
                    source.span(span.start.byte_offset + i..span.start.byte_offset + close + 2),
                    "Invalid inline link syntax",
                ));
                let Some(end) = unescaped_close(raw, close + 2, b')') else {
                    return;
                };
                i = end + 1;
                continue;
            }
        }
        i += raw[i..].chars().next().map_or(1, char::len_utf8);
    }
}

fn rejected_token_end(raw: &str, start: usize) -> Option<usize> {
    let remaining = &raw[start..];
    if remaining.starts_with("~~") {
        let mut i = start + 2;
        while i < raw.len() {
            if raw.as_bytes()[i] == b'\\'
                && raw
                    .as_bytes()
                    .get(i + 1)
                    .is_some_and(u8::is_ascii_punctuation)
            {
                i += 2;
            } else if raw[i..].starts_with("~~") {
                return Some(i + 2);
            } else {
                i += raw[i..].chars().next().map_or(1, char::len_utf8);
            }
        }
        None
    } else if remaining.starts_with('<') {
        unescaped_close(raw, start + 1, b'>').map(|end| end + 1)
    } else if remaining.starts_with("![") || remaining.starts_with("[^") {
        let close = unescaped_close(raw, start + 2, b']')?;
        match raw.as_bytes().get(close + 1) {
            Some(b'(') => unescaped_close(raw, close + 2, b')').map(|end| end + 1),
            Some(b'[') => unescaped_close(raw, close + 2, b']').map(|end| end + 1),
            _ => Some(close + 1),
        }
    } else if remaining.starts_with("::") {
        Some(start + remaining.find(['\r', '\n']).unwrap_or(remaining.len()))
    } else {
        Some(
            start
                + remaining
                    .find(char::is_whitespace)
                    .unwrap_or(remaining.len()),
        )
    }
}

fn unescaped_close(raw: &str, mut i: usize, delimiter: u8) -> Option<usize> {
    let bytes = raw.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            i += 2;
        } else if bytes[i] == delimiter {
            return Some(i);
        } else {
            i += 1;
        }
    }
    None
}
