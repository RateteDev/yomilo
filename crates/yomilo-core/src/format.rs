use crate::*;

pub fn format(document: &StructuredDocument) -> String {
    let mut parts = Vec::new();
    if let Some(frontmatter) = document.frontmatter() {
        let marker = match frontmatter.value.format {
            FrontmatterFormat::Yaml => "---",
            FrontmatterFormat::Toml => "+++",
        };
        parts.push(format!("{marker}\n{}\n{marker}", frontmatter.value.value));
    }
    parts.extend(
        document
            .blocks()
            .iter()
            .map(|block| block_text(&block.value)),
    );
    if parts.is_empty() {
        String::new()
    } else {
        format!("{}\n", parts.join("\n\n"))
    }
}

fn blocks_text(blocks: &[Located<BlockNode>]) -> String {
    blocks
        .iter()
        .map(|block| block_text(&block.value))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn block_text(block: &BlockNode) -> String {
    match block {
        BlockNode::Heading(heading) => {
            let fold = match heading.fold {
                FoldIntent::None => "",
                FoldIntent::InitiallyClosed => "> ",
                FoldIntent::InitiallyOpen => "v ",
            };
            let mut content = inlines(&heading.content, false, false);
            if heading.level == HeadingLevel::H3
                && heading.fold == FoldIntent::None
                && content.starts_with("v ")
            {
                content.replace_range(..1, "&#118;");
            }
            format!(
                "{} {fold}{content}",
                "#".repeat(usize::from(heading.level.depth()))
            )
        }
        BlockNode::Paragraph(paragraph) => inlines(&paragraph.content, false, true),
        BlockNode::List(list) => list_text(list),
        BlockNode::Quote(quote) => blocks_text(&quote.blocks)
            .split('\n')
            .map(|line| {
                if line.is_empty() {
                    ">".to_owned()
                } else {
                    format!("> {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        BlockNode::CodeBlock(code) => code_text(code, None),
        BlockNode::Table(table) => {
            let mut rows = vec![row_text(&table.header.value)];
            rows.push(format!(
                "| {} |",
                table
                    .align
                    .iter()
                    .map(|align| match align {
                        ColumnAlignment::Unspecified => "---",
                        ColumnAlignment::Left => ":---",
                        ColumnAlignment::Center => ":---:",
                        ColumnAlignment::Right => "---:",
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            rows.extend(table.rows.iter().map(|row| row_text(&row.value)));
            rows.join("\n")
        }
        BlockNode::ThematicBreak => "---".to_owned(),
        BlockNode::Callout(callout) => {
            let attributes = vec![
                string_attribute("type", callout.kind.as_str()),
                string_attribute("label", &callout.label),
            ];
            block_directive("callout", attributes, &blocks_text(&callout.blocks))
        }
        BlockNode::CodeGroup(group) => block_directive(
            "code-group",
            Vec::new(),
            &group
                .entries
                .iter()
                .map(|entry| code_text(&entry.value.code, Some(&entry.value.label)))
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
        BlockNode::Svg(svg) => {
            let mut attributes = Vec::new();
            optional_attribute(&mut attributes, "alt", svg.alt.as_deref());
            optional_attribute(&mut attributes, "caption", svg.caption.as_deref());
            if svg.zoom {
                attributes.push("zoom".to_owned());
            }
            block_directive("svg", attributes, svg.body.as_str())
        }
        BlockNode::Image(image) => {
            let mut attributes = vec![string_attribute("src", &image.src)];
            optional_attribute(&mut attributes, "alt", image.alt.as_deref());
            ratio_attribute(&mut attributes, image.ratio);
            optional_attribute(&mut attributes, "caption", image.caption.as_deref());
            if image.zoom {
                attributes.push("zoom".to_owned());
            }
            format!("::image{}", attribute_block(attributes))
        }
        BlockNode::Video(video) => {
            let mut attributes = vec![string_attribute("src", &video.src)];
            ratio_attribute(&mut attributes, video.ratio);
            optional_attribute(&mut attributes, "caption", video.caption.as_deref());
            if video.loop_playback {
                attributes.push("loop".to_owned());
            }
            format!("::video{}", attribute_block(attributes))
        }
        BlockNode::UrlCard(card) => {
            format!("::url-card{{{}}}", string_attribute("href", &card.href))
        }
        BlockNode::TweetCard(card) => {
            format!("::tweet-card{{{}}}", string_attribute("href", &card.href))
        }
    }
}

fn list_text(list: &List) -> String {
    let mut items = Vec::new();
    for (index, item) in list.items.iter().enumerate() {
        let prefix = match list.kind {
            ListKind::Unordered => "- ".to_owned(),
            ListKind::Ordered => format!("{}. ", index + 1),
        };
        let separator = if item.value.spread { "\n\n" } else { "\n" };
        let content = item
            .value
            .blocks
            .iter()
            .map(|block| block_text(&block.value))
            .collect::<Vec<_>>()
            .join(separator);
        let indent = " ".repeat(prefix.len());
        let mut lines = content.split('\n');
        let mut text = format!("{prefix}{}", lines.next().unwrap_or(""));
        for line in lines {
            text.push('\n');
            if !line.is_empty() {
                text.push_str(&indent);
                text.push_str(line);
            }
        }
        items.push(text);
    }
    items.join(if list.spread { "\n\n" } else { "\n" })
}

fn inlines(nodes: &[Located<InlineNode>], table: bool, soft_breaks: bool) -> String {
    let mut text = String::new();
    for (index, node) in nodes.iter().enumerate() {
        match &node.value {
            InlineNode::Text(value) => text.push_str(&escape_text(
                value,
                index == 0,
                index + 1 == nodes.len(),
                !table,
                soft_breaks,
            )),
            InlineNode::Emphasis(children) => {
                text.push_str(&format!("*{}*", inlines(children, table, soft_breaks)))
            }
            InlineNode::Strong(children) => {
                text.push_str(&format!("**{}**", inlines(children, table, soft_breaks)))
            }
            InlineNode::Code(value) => {
                let value = if table {
                    value.replace('|', "\\|")
                } else {
                    value.clone()
                };
                if value.starts_with(' ')
                    && value.ends_with(' ')
                    && value.chars().any(|ch| ch != ' ')
                {
                    text.push_str(&format!("` {value} `"));
                } else {
                    text.push_str(&format!("`{value}`"));
                }
            }
            InlineNode::Link(link) => {
                let destination = link.href.chars().fold(String::new(), |mut result, ch| {
                    if matches!(ch, '\\' | '(' | ')' | '<' | '>' | '&' | '"' | '\'' | '|') {
                        result.push('\\');
                    }
                    result.push(ch);
                    result
                });
                text.push_str(&format!(
                    "[{}]({destination})",
                    inlines(&link.content, table, soft_breaks)
                ));
            }
        }
    }
    text
}

fn escape_text(
    value: &str,
    first: bool,
    last: bool,
    block_syntax: bool,
    soft_breaks: bool,
) -> String {
    let mut result = String::new();
    let count = value.chars().count();
    let mut line_start = 0;
    for (index, (byte_index, ch)) in value.char_indices().enumerate() {
        if ch == '\n' {
            // 空行や単行構文へ展開される改行は、文字参照で本文の意味を保つ。
            if !soft_breaks
                || (first && index == 0)
                || (last && index + 1 == count)
                || value[..byte_index].ends_with('\n')
                || value[byte_index + 1..].starts_with('\n')
            {
                result.push_str("&#10;");
            } else {
                result.push('\n');
            }
            line_start = byte_index + 1;
        } else if ch == ' '
            && ((first && index == 0)
                || (last && index + 1 == count)
                || value[..byte_index].ends_with('\n')
                || value[byte_index + 1..].starts_with('\n'))
        {
            result.push_str("&#32;");
        } else if ch == '\t' {
            result.push_str("&#9;");
        } else {
            let prefix = &value[line_start..byte_index];
            let escape_line_start = first || line_start != 0;
            let block_marker = block_syntax
                && (escape_line_start && prefix.is_empty() && matches!(ch, '+' | '-')
                    || ch == '=' && line_start != 0 && prefix.is_empty()
                    || (ch == '.' || ch == ')' && !prefix.is_empty())
                        && escape_line_start
                        && prefix.bytes().all(|byte| byte.is_ascii_digit())
                        && value[byte_index + 1..].starts_with(' '));
            if matches!(
                ch,
                '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '!' | '|' | '~' | ':'
            ) || (ch == '#' && block_syntax)
                || block_marker
                || (ch == '.' && value[..byte_index].ends_with("www"))
            {
                result.push('\\');
            }
            result.push(ch);
        }
    }
    result
}

fn row_text(row: &TableRow) -> String {
    format!(
        "| {} |",
        row.cells
            .iter()
            .map(|cell| inlines(&cell.value.content, true, false))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}

fn code_text(code: &CodeBlock, label: Option<&str>) -> String {
    let mut maximum = 0;
    let mut run = 0;
    for byte in code.value.bytes() {
        if byte == b'`' {
            run += 1;
            maximum = maximum.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat((maximum + 1).max(3));
    let label = label.map_or(String::new(), |label| {
        format!(" {}", escape_metadata(label))
    });
    if code.value.is_empty() {
        format!("{fence}{}{label}\n{fence}", code.language)
    } else {
        format!("{fence}{}{label}\n{}\n{fence}", code.language, code.value)
    }
}

fn escape_metadata(value: &str) -> String {
    value.chars().fold(String::new(), |mut result, ch| {
        match ch {
            '`' => result.push_str("&#96;"),
            '&' => result.push_str("&#38;"),
            '\\' => result.push_str("&#92;"),
            _ => result.push(ch),
        }
        result
    })
}

fn string_attribute(name: &str, value: &str) -> String {
    format!(
        "{name}=\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn optional_attribute(attributes: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        attributes.push(string_attribute(name, value));
    }
}

fn ratio_attribute(attributes: &mut Vec<String>, ratio: Option<Ratio>) {
    if let Some(ratio) = ratio {
        attributes.push(string_attribute(
            "ratio",
            &format!("{}/{}", ratio.width(), ratio.height()),
        ));
    }
}

fn attribute_block(attributes: Vec<String>) -> String {
    if attributes.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", attributes.join(" "))
    }
}

fn block_directive(name: &str, attributes: Vec<String>, body: &str) -> String {
    if body.is_empty() {
        format!(":::{name}{}\n:::", attribute_block(attributes))
    } else {
        format!(":::{name}{}\n{body}\n:::", attribute_block(attributes))
    }
}
