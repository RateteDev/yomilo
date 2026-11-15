use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct SourcePoint {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct SourceSpan {
    pub start: SourcePoint,
    pub end: SourcePoint,
}

impl SourceSpan {
    pub fn byte_range(&self) -> Range<usize> {
        self.start.byte_offset..self.end.byte_offset
    }
}

pub(crate) struct Source<'a> {
    pub text: &'a str,
    line_starts: Vec<usize>,
}

#[derive(Clone, Copy)]
pub(crate) struct Line<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let bytes = text.as_bytes();
        let mut line_starts = vec![0];
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\r' {
                i += usize::from(bytes.get(i + 1) == Some(&b'\n'));
                line_starts.push(i + 1);
            } else if bytes[i] == b'\n' {
                line_starts.push(i + 1);
            }
            i += 1;
        }
        Self { text, line_starts }
    }

    pub fn point(&self, offset: usize) -> SourcePoint {
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        SourcePoint {
            byte_offset: offset,
            line: line_index + 1,
            column: self.text[self.line_starts[line_index]..offset]
                .chars()
                .count()
                + 1,
        }
    }

    pub fn span(&self, range: Range<usize>) -> SourceSpan {
        SourceSpan {
            start: self.point(range.start),
            end: self.point(range.end),
        }
    }

    pub fn line_start(&self, offset: usize) -> usize {
        self.line_starts[self.line_starts.partition_point(|start| *start <= offset) - 1]
    }

    pub fn is_line_start(&self, offset: usize) -> bool {
        self.line_starts.binary_search(&offset).is_ok()
    }

    pub fn whole_span(&self) -> SourceSpan {
        self.span(0..self.text.len())
    }

    pub fn lines(&self, range: Range<usize>) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        let mut start = range.start;
        let bytes = self.text.as_bytes();
        let mut i = start;
        while i < range.end {
            if matches!(bytes[i], b'\n' | b'\r') {
                let content_end = i;
                if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') && i + 1 < range.end {
                    i += 1;
                }
                i += 1;
                lines.push(Line {
                    text: &self.text[start..content_end],
                    start,
                    end: i,
                });
                start = i;
            } else {
                i += 1;
            }
        }
        if start < range.end {
            lines.push(Line {
                text: &self.text[start..range.end],
                start,
                end: range.end,
            });
        }
        lines
    }
}

pub(crate) fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
