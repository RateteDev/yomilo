use crate::diagnostic::DiagnosticBag;
use crate::source::{Source, normalize_line_endings};
use crate::*;
use ::markdown::{Constructs, ParseOptions, mdast};
use std::ops::Range;

#[derive(Clone, Copy, Default)]
pub(crate) struct Context {
    callout: bool,
    quote: bool,
    inside_link: bool,
    container_indent: usize,
}

impl Context {
    pub fn callout() -> Self {
        Self {
            callout: true,
            ..Self::default()
        }
    }
}

pub(crate) struct Parsed {
    pub frontmatter: Option<Located<Frontmatter>>,
    pub blocks: Vec<Located<BlockNode>>,
}

pub(crate) struct Mapper<'a, 's> {
    source: &'s Source<'a>,
    range: Range<usize>,
}

pub(crate) fn tree<'a, 's>(
    source: &'s Source<'a>,
    range: Range<usize>,
    frontmatter: bool,
) -> Result<(Vec<mdast::Node>, Mapper<'a, 's>), Diagnostic> {
    let options = ParseOptions {
        constructs: Constructs {
            frontmatter,
            ..Constructs::gfm()
        },
        gfm_strikethrough_single_tilde: false,
        ..ParseOptions::default()
    };
    let node = ::markdown::to_mdast(&source.text[range.clone()], &options).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::InvalidSyntax,
            DiagnosticCode::ParserFailure,
            source.span(range.clone()),
            error.reason,
        )
    })?;
    let mdast::Node::Root(root) = node else {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::ParserFailure,
            source.span(range),
            "Markdown parser did not return a document root",
        ));
    };
    const MAX_NESTING: usize = 64;
    let mut pending: Vec<_> = root.children.iter().map(|node| (node, 1)).collect();
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_NESTING {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::ResourceLimit,
                source.span(range.clone()),
                "Markdown nesting exceeds the supported limit",
            ));
        }
        if let Some(children) = node.children() {
            pending.extend(children.iter().map(|child| (child, depth + 1)));
        }
    }
    Ok((root.children, Mapper { source, range }))
}

pub(crate) fn parse(
    source: &Source<'_>,
    range: Range<usize>,
    context: Context,
    diagnostics: &mut DiagnosticBag,
) -> Result<Parsed, Diagnostic> {
    let frontmatter_enabled = range.start == 0 && !context.callout;
    let (nodes, mapper) = tree(source, range, frontmatter_enabled)?;
    let mut parsed = Parsed {
        frontmatter: None,
        blocks: Vec::new(),
    };
    for (index, node) in nodes.into_iter().enumerate() {
        let span = mapper.span(&node)?;
        let frontmatter = match node {
            mdast::Node::Yaml(node) => Some(Frontmatter {
                format: FrontmatterFormat::Yaml,
                value: normalize_line_endings(&node.value),
            }),
            mdast::Node::Toml(node) => Some(Frontmatter {
                format: FrontmatterFormat::Toml,
                value: normalize_line_endings(&node.value),
            }),
            node => {
                let block = mapper.block(node, context, diagnostics);
                if let Some(block) = diagnostics.capture(block) {
                    parsed.blocks.push(block);
                }
                None
            }
        };
        if let Some(value) = frontmatter {
            if index != 0 || !frontmatter_enabled {
                diagnostics.push(mapper.unsupported(
                    span,
                    "Frontmatter is allowed only at the start of a document",
                ));
                continue;
            }
            parsed.frontmatter = Some(Located { value, span });
        }
    }
    Ok(parsed)
}

impl Mapper<'_, '_> {
    pub fn span(&self, node: &mdast::Node) -> Result<SourceSpan, Diagnostic> {
        let position = node.position().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::MissingPosition,
                self.source.span(self.range.clone()),
                "Markdown node has no source position",
            )
        })?;
        let start = self.range.start + position.start.offset;
        let end = self.range.start + position.end.offset;
        if start > end
            || end > self.range.end
            || !self.source.text.is_char_boundary(start)
            || !self.source.text.is_char_boundary(end)
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::MissingPosition,
                self.source.span(self.range.clone()),
                "Markdown node position is outside its source",
            ));
        }
        Ok(self.source.span(start..end))
    }

    fn unsupported(&self, span: SourceSpan, message: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticClass::Unsupported,
            DiagnosticCode::UnsupportedSyntax,
            span,
            message,
        )
    }

    fn blocks(
        &self,
        nodes: Vec<mdast::Node>,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Vec<Located<BlockNode>> {
        let mut result = Vec::new();
        for node in nodes {
            let block = self.block(node, context, diagnostics);
            if let Some(block) = diagnostics.capture(block) {
                result.push(block);
            }
        }
        result
    }

    fn block(
        &self,
        node: mdast::Node,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Result<Located<BlockNode>, Diagnostic> {
        let span = self.span(&node)?;
        if context.quote && !matches!(node, mdast::Node::Paragraph(_) | mdast::Node::List(_)) {
            return Err(self.unsupported(span, "Quotes may contain only paragraphs and lists"));
        }
        let value = match node {
            mdast::Node::Heading(node) => {
                if context.callout {
                    return Err(
                        self.unsupported(span, "Headings inside callouts are not supported")
                    );
                }
                let raw = self.source.text[span.byte_range()].trim_start_matches([' ', '\t']);
                let level = match node.depth {
                    1 => HeadingLevel::H1,
                    2 => HeadingLevel::H2,
                    3 => HeadingLevel::H3,
                    _ => {
                        return Err(
                            self.unsupported(span, "Only h1, h2, and h3 headings are supported")
                        );
                    }
                };
                let count = raw.bytes().take_while(|b| *b == b'#').count();
                if count != usize::from(node.depth) {
                    return Err(self.unsupported(span, "Setext headings are not supported"));
                }
                let heading_text = raw[count..].trim_start_matches([' ', '\t']);
                let fold = if level == HeadingLevel::H3 && heading_text.starts_with("> ") {
                    FoldIntent::InitiallyClosed
                } else if level == HeadingLevel::H3 && heading_text.starts_with("v ") {
                    FoldIntent::InitiallyOpen
                } else {
                    FoldIntent::None
                };
                let mut content = self.inlines(node.children, context, diagnostics);
                if fold != FoldIntent::None {
                    if let Some(Located {
                        value: InlineNode::Text(text),
                        span,
                    }) = content.first_mut()
                        && (text.starts_with("> ") || text.starts_with("v "))
                    {
                        text.drain(..2);
                        span.start = self.source.point(span.start.byte_offset + 2);
                    }
                    content.retain(
                        |node| !matches!(&node.value, InlineNode::Text(text) if text.is_empty()),
                    );
                }
                BlockNode::Heading(Heading {
                    level,
                    fold,
                    content,
                    id: None,
                })
            }
            mdast::Node::Paragraph(node) => BlockNode::Paragraph(Paragraph {
                content: self.inlines(node.children, context, diagnostics),
            }),
            mdast::Node::List(node) => {
                let mut items = Vec::new();
                for (index, item) in node.children.into_iter().enumerate() {
                    let item = self.list_item(item, index, node.ordered, context, diagnostics);
                    if let Some(item) = diagnostics.capture(item) {
                        items.push(item);
                    }
                }
                BlockNode::List(List {
                    kind: if node.ordered {
                        ListKind::Ordered
                    } else {
                        ListKind::Unordered
                    },
                    spread: node.spread,
                    items,
                })
            }
            mdast::Node::Blockquote(node) => {
                let blocks = self.blocks(
                    node.children,
                    Context {
                        quote: true,
                        ..context
                    },
                    diagnostics,
                );
                BlockNode::Quote(Quote { blocks })
            }
            mdast::Node::Code(node) => {
                BlockNode::CodeBlock(self.code(node, span, false, context.container_indent)?)
            }
            mdast::Node::Table(node) => {
                if node.children.is_empty() {
                    return Err(self.unsupported(span, "A table requires a header row"));
                }
                let mut rows = Vec::new();
                let mut header = None;
                let mut header_error = None;
                for (index, row) in node.children.into_iter().enumerate() {
                    match self.table_row(row, node.align.len(), context, diagnostics) {
                        Ok(row) if index == 0 => header = Some(row),
                        Ok(row) => rows.push(row),
                        Err(diagnostic) => {
                            if index == 0 {
                                header_error = Some(diagnostic.clone());
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                }
                let header =
                    header.ok_or_else(|| header_error.expect("invalid header has a diagnostic"))?;
                let align = node
                    .align
                    .into_iter()
                    .map(|align| match align {
                        mdast::AlignKind::None => ColumnAlignment::Unspecified,
                        mdast::AlignKind::Left => ColumnAlignment::Left,
                        mdast::AlignKind::Center => ColumnAlignment::Center,
                        mdast::AlignKind::Right => ColumnAlignment::Right,
                    })
                    .collect();
                BlockNode::Table(Table {
                    id: ReferenceId(String::new()),
                    align,
                    header,
                    rows,
                })
            }
            mdast::Node::ThematicBreak(_) => {
                if self.source.text[span.byte_range()].trim() != "---" {
                    return Err(self.unsupported(span, "Thematic breaks must use '---'"));
                }
                BlockNode::ThematicBreak
            }
            mdast::Node::FootnoteDefinition(_) => {
                return Err(self.unsupported(span, "Footnotes are not supported"));
            }
            mdast::Node::Definition(_) => {
                return Err(self.unsupported(span, "Reference link definitions are not supported"));
            }
            mdast::Node::Html(_) => {
                return Err(self.unsupported(span, "Raw HTML and JSX are not supported"));
            }
            mdast::Node::Root(_)
            | mdast::Node::MdxJsxFlowElement(_)
            | mdast::Node::MdxjsEsm(_)
            | mdast::Node::Toml(_)
            | mdast::Node::Yaml(_)
            | mdast::Node::Break(_)
            | mdast::Node::InlineCode(_)
            | mdast::Node::InlineMath(_)
            | mdast::Node::Delete(_)
            | mdast::Node::Emphasis(_)
            | mdast::Node::MdxTextExpression(_)
            | mdast::Node::FootnoteReference(_)
            | mdast::Node::Image(_)
            | mdast::Node::ImageReference(_)
            | mdast::Node::MdxJsxTextElement(_)
            | mdast::Node::Link(_)
            | mdast::Node::LinkReference(_)
            | mdast::Node::Strong(_)
            | mdast::Node::Text(_)
            | mdast::Node::Math(_)
            | mdast::Node::MdxFlowExpression(_)
            | mdast::Node::TableRow(_)
            | mdast::Node::TableCell(_)
            | mdast::Node::ListItem(_) => {
                return Err(Diagnostic::new(
                    DiagnosticClass::Unsupported,
                    DiagnosticCode::UnsupportedNode,
                    span,
                    "Node is not supported in block content",
                ));
            }
        };
        Ok(Located { value, span })
    }

    fn list_item(
        &self,
        node: mdast::Node,
        index: usize,
        ordered: bool,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Result<Located<ListItem>, Diagnostic> {
        let span = self.span(&node)?;
        let mdast::Node::ListItem(item) = node else {
            return Err(self.unsupported(span, "Invalid list item"));
        };
        if item.checked.is_some() {
            return Err(self.unsupported(span, "Task lists are not supported"));
        }
        let raw = &self.source.text[span.byte_range()];
        let marker_width = if ordered {
            let count = raw.bytes().take_while(u8::is_ascii_digit).count();
            let number = raw[..count].parse::<usize>().ok();
            if number != Some(index + 1) || raw.as_bytes().get(count) != Some(&b'.') {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    DiagnosticCode::InvalidListNumber,
                    span,
                    "Ordered list items must use consecutive numbers starting at 1 with '.'",
                ));
            }
            count + 1
        } else if !raw.starts_with("- ") {
            return Err(self.unsupported(span, "Unordered lists must use '- '"));
        } else {
            1
        };
        let spaces = raw[marker_width..]
            .bytes()
            .take_while(|byte| *byte == b' ')
            .count();
        let padding = if spaces > 4 { 1 } else { spaces.max(1) };
        let context = Context {
            container_indent: span.start.column - 1 + marker_width + padding,
            ..context
        };
        Ok(Located {
            value: ListItem {
                spread: item.spread,
                blocks: self.blocks(item.children, context, diagnostics),
            },
            span,
        })
    }

    fn table_row(
        &self,
        node: mdast::Node,
        columns: usize,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Result<Located<TableRow>, Diagnostic> {
        let span = self.span(&node)?;
        let mdast::Node::TableRow(row) = node else {
            return Err(self.unsupported(span, "Invalid table row"));
        };
        let size = row.children.len();
        let mut cells = Vec::new();
        for cell in row.children {
            let cell_span = self.span(&cell)?;
            let mdast::Node::TableCell(cell) = cell else {
                diagnostics.push(self.unsupported(cell_span, "Invalid table cell"));
                continue;
            };
            cells.push(Located {
                value: TableCell {
                    content: self.inlines(cell.children, context, diagnostics),
                },
                span: cell_span,
            });
        }
        if size != columns || table_columns(&self.source.text[span.byte_range()]) != columns {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::InvalidTable,
                span,
                "Every table row must have the same number of columns",
            ));
        }
        Ok(Located {
            value: TableRow { cells },
            span,
        })
    }

    pub fn code(
        &self,
        node: mdast::Code,
        span: SourceSpan,
        metadata: bool,
        container_indent: usize,
    ) -> Result<CodeBlock, Diagnostic> {
        let lines = self.source.lines(span.byte_range());
        let raw = lines
            .first()
            .map_or("", |line| line.text.trim_start_matches(' '));
        let count = raw.bytes().take_while(|b| *b == b'`').count();
        if count < 3 {
            return Err(Diagnostic::new(
                DiagnosticClass::Unsupported,
                DiagnosticCode::InvalidCodeFence,
                span,
                "Code blocks must use backtick fences",
            ));
        }
        let closing_raw = lines.last().map_or("", |line| line.text);
        let closing_indent = closing_raw.bytes().take_while(|byte| *byte == b' ').count();
        let closing = &closing_raw[closing_indent..];
        let close_count = closing.bytes().take_while(|b| *b == b'`').count();
        if lines.len() < 2
            || close_count < count
            || closing_indent < container_indent
            || closing_indent > container_indent + 3
            || !closing[close_count..].trim().is_empty()
        {
            return Err(Diagnostic::new(
                DiagnosticClass::InvalidSyntax,
                DiagnosticCode::UnclosedCodeFence,
                self.source.span(
                    lines
                        .first()
                        .map_or(span.byte_range(), |line| line.start..line.end),
                ),
                "Code fence requires an explicit closing fence",
            ));
        }
        let language = node.lang.filter(|value| !value.is_empty()).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::MissingCodeLanguage,
                span,
                "Code blocks require a language name",
            )
        })?;
        if !language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-' | b'.' | b'#'))
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                DiagnosticCode::InvalidCodeFence,
                span,
                "Code language must be a single identifier",
            ));
        }
        if !metadata && node.meta.is_some() {
            return Err(Diagnostic::new(
                DiagnosticClass::Unsupported,
                DiagnosticCode::UnexpectedCodeMetadata,
                span,
                "Code metadata is allowed only inside code groups",
            ));
        }
        Ok(CodeBlock {
            id: ReferenceId(String::new()),
            language,
            value: normalize_line_endings(&node.value),
        })
    }

    fn inlines(
        &self,
        nodes: Vec<mdast::Node>,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Vec<Located<InlineNode>> {
        let mut result = Vec::new();
        for node in nodes {
            let node = match node {
                // GFMの自動リンクは明示リンクの表示文字内でも生成される。
                mdast::Node::Link(node) if context.inside_link => {
                    for child in self.inlines(node.children, context, diagnostics) {
                        push_inline(&mut result, child);
                    }
                    continue;
                }
                node => node,
            };
            let node = self.inline(node, context, diagnostics);
            if let Some(node) = diagnostics.capture(node) {
                push_inline(&mut result, node);
            }
        }
        result
    }

    fn inline(
        &self,
        node: mdast::Node,
        context: Context,
        diagnostics: &mut DiagnosticBag,
    ) -> Result<Located<InlineNode>, Diagnostic> {
        let span = self.span(&node)?;
        let value = match node {
            mdast::Node::Text(node) => {
                crate::validate::text_syntax(self.source, span, context.inside_link, diagnostics);
                InlineNode::Text(normalize_line_endings(&node.value))
            }
            mdast::Node::Emphasis(node) => {
                if !self.source.text[span.byte_range()].starts_with('*') {
                    return Err(self.unsupported(span, "Underscore emphasis is not supported"));
                }
                InlineNode::Emphasis(self.inlines(node.children, context, diagnostics))
            }
            mdast::Node::Strong(node) => {
                if !self.source.text[span.byte_range()].starts_with("**") {
                    return Err(self.unsupported(span, "Underscore emphasis is not supported"));
                }
                InlineNode::Strong(self.inlines(node.children, context, diagnostics))
            }
            mdast::Node::InlineCode(node) => {
                let raw = &self.source.text[span.byte_range()];
                if raw.bytes().take_while(|b| *b == b'`').count() != 1 {
                    return Err(
                        self.unsupported(span, "Inline code must use a single backtick delimiter")
                    );
                }
                InlineNode::Code(normalize_line_endings(&node.value).replace('\n', " "))
            }
            mdast::Node::Link(node) => {
                if !self.source.text[span.byte_range()].starts_with('[') {
                    return Err(self.unsupported(span, "Bare URLs and autolinks are not supported"));
                }
                if node.title.is_some() {
                    return Err(self.unsupported(span, "Link titles are not supported"));
                }
                let reference = crate::validate::reference(&node.url, span, true, false);
                let content = self.inlines(
                    node.children,
                    Context {
                        inside_link: true,
                        ..context
                    },
                    diagnostics,
                );
                reference?;
                InlineNode::Link(Link {
                    content,
                    href: node.url,
                })
            }
            mdast::Node::Break(_) => {
                return Err(self.unsupported(span, "Hard line breaks are not supported"));
            }
            mdast::Node::Delete(_) => {
                return Err(self.unsupported(span, "Strikethrough is not supported"));
            }
            mdast::Node::FootnoteReference(_) => {
                return Err(self.unsupported(span, "Footnotes are not supported"));
            }
            mdast::Node::LinkReference(_) => {
                return Err(self.unsupported(span, "Reference links are not supported"));
            }
            mdast::Node::Image(_) | mdast::Node::ImageReference(_) => {
                return Err(self.unsupported(span, "Markdown images are not supported"));
            }
            mdast::Node::Html(_) => {
                return Err(self.unsupported(span, "Raw HTML and JSX are not supported"));
            }
            mdast::Node::Root(_)
            | mdast::Node::Blockquote(_)
            | mdast::Node::FootnoteDefinition(_)
            | mdast::Node::MdxJsxFlowElement(_)
            | mdast::Node::List(_)
            | mdast::Node::MdxjsEsm(_)
            | mdast::Node::Toml(_)
            | mdast::Node::Yaml(_)
            | mdast::Node::InlineMath(_)
            | mdast::Node::MdxTextExpression(_)
            | mdast::Node::MdxJsxTextElement(_)
            | mdast::Node::Code(_)
            | mdast::Node::Math(_)
            | mdast::Node::MdxFlowExpression(_)
            | mdast::Node::Heading(_)
            | mdast::Node::Table(_)
            | mdast::Node::ThematicBreak(_)
            | mdast::Node::TableRow(_)
            | mdast::Node::TableCell(_)
            | mdast::Node::ListItem(_)
            | mdast::Node::Definition(_)
            | mdast::Node::Paragraph(_) => {
                return Err(Diagnostic::new(
                    DiagnosticClass::Unsupported,
                    DiagnosticCode::UnsupportedNode,
                    span,
                    "Node is not supported in inline content",
                ));
            }
        };
        Ok(Located { value, span })
    }
}

fn push_inline(nodes: &mut Vec<Located<InlineNode>>, node: Located<InlineNode>) {
    if let Some(previous) = nodes.last_mut() {
        match (&mut previous.value, &node.value) {
            (InlineNode::Text(a), InlineNode::Text(b)) => {
                a.push_str(b);
                previous.span.end = node.span.end;
                return;
            }
            (InlineNode::Emphasis(a), InlineNode::Emphasis(b))
            | (InlineNode::Strong(a), InlineNode::Strong(b)) => {
                for child in b {
                    push_inline(a, child.clone());
                }
                previous.span.end = node.span.end;
                return;
            }
            _ => {}
        }
    }
    nodes.push(node);
}

fn table_columns(raw: &str) -> usize {
    let raw = raw.trim();
    if raw == "|" {
        return 1;
    }
    let bytes = raw.as_bytes();
    let mut pipes = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            i += 2;
        } else {
            if bytes[i] == b'|' {
                pipes.push(i);
            }
            i += 1;
        }
    }
    let leading = pipes.first() == Some(&0);
    let trailing = pipes.last().is_some_and(|index| *index + 1 == bytes.len());
    pipes.len() + 1 - usize::from(leading) - usize::from(trailing)
}
