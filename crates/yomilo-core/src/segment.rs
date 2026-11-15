use crate::diagnostic::DiagnosticBag;
use crate::directive::scan::{Frontmatter, SourcePart, frontmatter, scan};
use crate::ids::IdCounters;
use crate::markdown::{self, Context};
use crate::source::Source;
use crate::{
    BlockNode, Diagnostic, DiagnosticClass, DiagnosticCode, Located, ModelVersion, SourceSpan,
    StructureError, SyntaxVersion,
};
use std::{error::Error, fmt, ops::Range};

const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct StructureVersions {
    pub syntax_version: u16,
    pub model_version: u16,
}

impl StructureVersions {
    pub fn current() -> Self {
        Self {
            syntax_version: SyntaxVersion::V0.number(),
            model_version: ModelVersion::V0.number(),
        }
    }
}

/// 原稿がこの先も続くかどうか。続く原稿では、最後の要素がまだ伸びうるため確定しない。
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum InputState {
    Partial,
    Complete,
}

/// 次の呼び出しへ引き継ぐ位置。呼び出し側は中身を読まずにそのまま渡す。
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Cursor {
    versions: StructureVersions,
    offset: usize,
    next_index: usize,
    ids: IdCounters,
}

impl Cursor {
    fn start() -> Self {
        Self {
            versions: StructureVersions::current(),
            offset: 0,
            next_index: 0,
            ids: IdCounters::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Segment {
    pub versions: StructureVersions,
    pub index: usize,
    pub content: SegmentContent,
    /// 原稿の中での位置。JSONを経ると中の要素の位置は失われるため、描画準備の誤りはこの位置で示す。
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum SegmentContent {
    Frontmatter(Located<crate::Frontmatter>),
    Block(Located<BlockNode>),
    Rejected(RejectedSegment),
}

/// 規約に反したセグメント。黙って捨てず、原文と診断を残して描画側へ見せ方を委ねる。
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RejectedSegment {
    pub source: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct SegmentBatch {
    /// 今回新しく確定したセグメントだけを確定順に並べる。
    pub segments: Vec<Segment>,
    /// まだ確定していない末尾の原文。
    pub pending: String,
    pub cursor: Cursor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SegmentError {
    InvalidCursor { message: String },
    Structure(StructureError),
}

impl fmt::Display for SegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCursor { message } => f.write_str(message),
            Self::Structure(error) => error.fmt(f),
        }
    }
}

impl Error for SegmentError {}

impl From<Diagnostic> for SegmentError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Structure(diagnostic.into())
    }
}

enum Content {
    Frontmatter(Located<crate::Frontmatter>),
    Block(Located<BlockNode>),
}

struct Element {
    range: Range<usize>,
    content: Option<Content>,
}

/// 原稿を文書の直下の要素ごとのセグメントに分け、確定したものだけを返す。
/// 確定したセグメントは自分の原文と参照IDの続きだけで決まり、後から届く入力では変わらない。
pub fn segment(
    source: &str,
    cursor: Option<&Cursor>,
    input: InputState,
) -> Result<SegmentBatch, SegmentError> {
    let mut cursor = cursor.cloned().unwrap_or_else(Cursor::start);
    if cursor.versions != StructureVersions::current() {
        return Err(SegmentError::InvalidCursor {
            message: format!(
                "The cursor was made for versions {:?}, but this engine uses {:?}",
                cursor.versions,
                StructureVersions::current()
            ),
        });
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::ResourceLimit,
            Source::new("").whole_span(),
            "Source exceeds the supported byte limit",
        )
        .into());
    }
    let start = cursor.offset;
    if start > source.len()
        || !source.is_char_boundary(start)
        || !(start == source.len() || Source::new(source).is_line_start(start))
    {
        return Err(SegmentError::InvalidCursor {
            message: "The cursor does not point to a segment boundary of this source".to_owned(),
        });
    }
    // 書きかけの最後の行は、続きが届くと前の要素の続きに変わりうる（`2`が`2.`になり前のリストへ加わるなど）。
    // 途中の原稿では、改行まで届いた行だけを解析する。
    let analyzed = match input {
        InputState::Complete => source,
        InputState::Partial => &source[..terminated_len(source).max(start)],
    };
    let text = Source::new(analyzed);
    // 先頭の`---`は、閉じる行が届くまで区切り線か前付けかが決まらない。
    if input == InputState::Partial
        && start == 0
        && matches!(
            frontmatter(&text.lines(0..analyzed.len())),
            Frontmatter::Open
        )
    {
        return Ok(SegmentBatch {
            segments: Vec::new(),
            pending: source.to_owned(),
            cursor,
        });
    }
    let mut diagnostics = DiagnosticBag::default();
    let elements = elements(&text, start, &mut diagnostics)?;
    let mut grouped = vec![Vec::new(); elements.len()];
    for diagnostic in diagnostics.into_values().map_err(SegmentError::Structure)? {
        let index = elements
            .partition_point(|element| element.range.start <= diagnostic.span.start.byte_offset)
            .checked_sub(1)
            .or_else(|| (!elements.is_empty()).then_some(0))
            .ok_or_else(|| {
                StructureError::InvariantViolation(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    DiagnosticCode::ParserFailure,
                    diagnostic.span,
                    "A diagnostic does not belong to any segment",
                ))
            })
            .map_err(SegmentError::Structure)?;
        grouped[index].push(diagnostic);
    }
    let confirmed = match input {
        InputState::Complete => elements.len(),
        InputState::Partial => elements.len().saturating_sub(1),
    };
    let next_offset = elements
        .get(confirmed)
        .map_or(analyzed.len(), |element| element.range.start);
    let mut segments = Vec::with_capacity(confirmed);
    for (element, mut diagnostics) in elements.into_iter().zip(grouped).take(confirmed) {
        let content = match element.content {
            Some(Content::Frontmatter(frontmatter)) if diagnostics.is_empty() => {
                Some(SegmentContent::Frontmatter(frontmatter))
            }
            Some(Content::Block(mut block)) if diagnostics.is_empty() => {
                match crate::ids::assign(&mut block, &mut cursor.ids) {
                    Ok(()) => Some(SegmentContent::Block(block)),
                    Err(diagnostic) => {
                        diagnostics.push(diagnostic);
                        None
                    }
                }
            }
            _ => None,
        };
        if content.is_none() && diagnostics.is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::UnsupportedNode,
                text.span(element.range.clone()),
                "This element could not be structured",
            ));
        }
        segments.push(Segment {
            versions: cursor.versions,
            index: cursor.next_index,
            content: content.unwrap_or_else(|| {
                SegmentContent::Rejected(RejectedSegment {
                    source: source[element.range.clone()].to_owned(),
                    diagnostics,
                })
            }),
            span: text.span(element.range),
        });
        cursor.next_index += 1;
    }
    if input == InputState::Complete || confirmed > 0 {
        cursor.offset = next_offset;
    }
    Ok(SegmentBatch {
        segments,
        pending: source[cursor.offset..].to_owned(),
        cursor,
    })
}

/// 改行まで届いた部分の長さ。末尾の`\r`は`\r\n`の前半かもしれないため、届いたとみなさない。
fn terminated_len(source: &str) -> usize {
    source
        .trim_end_matches('\r')
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1)
}

/// 要素の境目は`start`以降をまとめて解析して決め、各要素は自分の行だけを解析し直す。
/// 後ろの原稿の内容が前の要素の解釈へ及ばないようにするためである。
fn elements(
    text: &Source<'_>,
    start: usize,
    diagnostics: &mut DiagnosticBag,
) -> Result<Vec<Element>, SegmentError> {
    let mut elements = Vec::new();
    for part in scan(text, start, diagnostics) {
        match part {
            SourcePart::Markdown(range) => {
                let (nodes, mapper) = match markdown::tree(text, range.clone(), range.start == 0) {
                    Ok(tree) => tree,
                    Err(diagnostic) => {
                        let error = StructureError::from(diagnostic.clone());
                        if matches!(error, StructureError::InvariantViolation(_)) {
                            return Err(SegmentError::Structure(error));
                        }
                        diagnostics.push(diagnostic);
                        elements.push(Element {
                            range,
                            content: None,
                        });
                        continue;
                    }
                };
                for node in &nodes {
                    let span = mapper.span(node)?;
                    let range = text.line_start(span.start.byte_offset)..span.end.byte_offset;
                    let parsed =
                        markdown::parse(text, range.clone(), Context::default(), diagnostics);
                    let content = match diagnostics.capture(parsed) {
                        Some(parsed) => single(parsed, text.span(range.clone()))?,
                        None => None,
                    };
                    elements.push(Element { range, content });
                }
            }
            SourcePart::Directive { range, directive } => {
                let content = directive
                    .and_then(|raw| {
                        let block = crate::directive::structure(text, raw, diagnostics);
                        diagnostics.capture(block).flatten()
                    })
                    .map(Content::Block);
                elements.push(Element { range, content });
            }
        }
    }
    Ok(elements)
}

fn single(parsed: markdown::Parsed, span: SourceSpan) -> Result<Option<Content>, SegmentError> {
    let mut blocks = parsed.blocks;
    match (parsed.frontmatter, blocks.len()) {
        (Some(frontmatter), 0) => Ok(Some(Content::Frontmatter(frontmatter))),
        (None, 0) => Ok(None),
        (None, 1) => Ok(blocks.pop().map(Content::Block)),
        _ => Err(SegmentError::Structure(StructureError::InvariantViolation(
            Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::ParserFailure,
                span,
                "A top-level element did not reparse as a single segment",
            ),
        ))),
    }
}
