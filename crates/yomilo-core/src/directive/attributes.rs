use crate::diagnostic::DiagnosticBag;
use crate::source::Source;
use crate::{Diagnostic, DiagnosticClass, DiagnosticCode, SourceSpan};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct Attribute {
    pub value: Option<String>,
    pub name_span: SourceSpan,
    pub span: SourceSpan,
    pub value_span: SourceSpan,
    invalid: Option<Diagnostic>,
}

pub(crate) struct Attributes(pub BTreeMap<String, Attribute>);

pub(crate) fn parse(
    source: &Source<'_>,
    start: usize,
    text: &str,
    diagnostics: &mut DiagnosticBag,
) -> Result<Attributes, Diagnostic> {
    let error = |range: std::ops::Range<usize>, message: &str| {
        Diagnostic::new(
            DiagnosticClass::InvalidSyntax,
            DiagnosticCode::AttributeSyntax,
            source.span(start + range.start..start + range.end),
            message,
        )
    };
    if !text.starts_with('{') || !text.ends_with('}') {
        return Err(error(0..text.len(), "Expected a complete attribute block"));
    }
    let bytes = text.as_bytes();
    let limit = bytes.len() - 1;
    let mut i = 1;
    let mut values: BTreeMap<String, Attribute> = BTreeMap::new();
    while i < limit {
        while i < limit && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i == limit {
            break;
        }
        let attr_start = i;
        if !bytes[i].is_ascii_lowercase() {
            return Err(error(
                i..i + text[i..].chars().next().map_or(1, char::len_utf8),
                "Invalid attribute name",
            ));
        }
        i += 1;
        while i < limit
            && (bytes[i].is_ascii_lowercase() || bytes[i].is_ascii_digit() || bytes[i] == b'-')
        {
            i += 1;
        }
        let name_end = i;
        let name = &text[attr_start..name_end];
        let mut value = None;
        let mut value_range = attr_start..name_end;
        if bytes.get(i) == Some(&b'=') {
            i += 1;
            let value_start = i;
            if i >= limit {
                return Err(error(attr_start..i, "Missing attribute value"));
            }
            if matches!(bytes[i], b'\'' | b'"') {
                let quote = bytes[i];
                i += 1;
                let mut decoded = String::new();
                let mut closed = false;
                while i < limit {
                    if bytes[i] == quote {
                        i += 1;
                        closed = true;
                        break;
                    }
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i >= limit || !matches!(bytes[i], b'\\' | b'\'' | b'"') {
                            return Err(error(value_start..i, "Invalid attribute escape"));
                        }
                        decoded.push(char::from(bytes[i]));
                        i += 1;
                    } else {
                        let ch = text[i..].chars().next().expect("valid UTF-8 boundary");
                        if ch.is_control() {
                            return Err(error(
                                value_start..i + ch.len_utf8(),
                                "Control characters are not allowed in attributes",
                            ));
                        }
                        decoded.push(ch);
                        i += ch.len_utf8();
                    }
                }
                if !closed {
                    return Err(error(value_start..limit, "Unclosed attribute quote"));
                }
                value = Some(decoded);
            } else {
                while i < limit && !matches!(bytes[i], b' ' | b'\t') {
                    let ch = text[i..].chars().next().expect("valid UTF-8 boundary");
                    if ch.is_control() || matches!(ch, '{' | '}' | '\'' | '"' | '\\') {
                        return Err(error(
                            value_start..i + ch.len_utf8(),
                            "Invalid unquoted attribute value",
                        ));
                    }
                    i += ch.len_utf8();
                }
                if i == value_start {
                    return Err(error(attr_start..i, "Missing attribute value"));
                }
                value = Some(text[value_start..i].to_owned());
            }
            value_range = value_start..i;
        }
        if i < limit && !matches!(bytes[i], b' ' | b'\t') {
            return Err(error(
                attr_start..i + text[i..].chars().next().map_or(1, char::len_utf8),
                "Attributes must be separated by whitespace",
            ));
        }
        let attribute = Attribute {
            value,
            name_span: source.span(start + attr_start..start + name_end),
            span: source.span(start + attr_start..start + i),
            value_span: source.span(start + value_range.start..start + value_range.end),
            invalid: None,
        };
        if let Some(previous) = values.get_mut(name) {
            let diagnostic = Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::DuplicateAttribute,
                attribute.name_span,
                format!("Duplicate attribute '{name}'"),
            )
            .related(previous.name_span, "First declaration");
            diagnostics.push(diagnostic.clone());
            previous.invalid.get_or_insert(diagnostic);
        } else {
            values.insert(name.to_owned(), attribute);
        }
    }
    Ok(Attributes(values))
}

impl Attributes {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    pub fn check_names(&self, names: &[&str], diagnostics: &mut DiagnosticBag) {
        for (name, attribute) in &self.0 {
            if !names.contains(&name.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticClass::Unsupported,
                    DiagnosticCode::UnknownAttribute,
                    attribute.name_span,
                    format!("Unknown attribute '{name}'"),
                ));
            }
        }
    }

    pub fn string(&self, name: &str) -> Result<Option<String>, Diagnostic> {
        self.0
            .get(name)
            .map(|attribute| {
                if let Some(diagnostic) = &attribute.invalid {
                    return Err(diagnostic.clone());
                }
                attribute.value.clone().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Semantic,
                        DiagnosticCode::AttributeType,
                        attribute.span,
                        format!("Attribute '{name}' requires a string value"),
                    )
                })
            })
            .transpose()
    }

    pub fn required(&self, name: &str, header: SourceSpan) -> Result<String, Diagnostic> {
        self.string(name)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::MissingAttribute,
                header,
                format!("Missing required attribute '{name}'"),
            )
        })
    }

    pub fn boolean(&self, name: &str) -> Result<bool, Diagnostic> {
        match self.0.get(name) {
            None => Ok(false),
            Some(attribute) => {
                if let Some(diagnostic) = &attribute.invalid {
                    return Err(diagnostic.clone());
                }
                match attribute.value.as_deref() {
                    None | Some("true") => Ok(true),
                    Some("false") => Ok(false),
                    Some(_) => Err(Diagnostic::new(
                        DiagnosticClass::Semantic,
                        DiagnosticCode::AttributeType,
                        attribute.value_span,
                        format!("Attribute '{name}' requires true or false"),
                    )),
                }
            }
        }
    }

    pub fn value_span(&self, name: &str, fallback: SourceSpan) -> SourceSpan {
        self.0
            .get(name)
            .map_or(fallback, |attribute| attribute.value_span)
    }
}
