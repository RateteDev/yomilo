use crate::*;

/// 参照IDの連番の続き。確定したセグメントの順に振るため、後から届く入力で前の番号は動かない。
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct IdCounters {
    section: usize,
    parent: Option<usize>,
    child: usize,
    callout: usize,
    code: usize,
    table: usize,
    svg: usize,
}

pub(crate) fn assign(
    block: &mut Located<BlockNode>,
    counters: &mut IdCounters,
) -> Result<(), Diagnostic> {
    visit(std::slice::from_mut(block), counters)
}

fn next(
    counter: &mut usize,
    prefix: &str,
    width: usize,
    span: SourceSpan,
) -> Result<ReferenceId, Diagnostic> {
    *counter = counter.checked_add(1).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Semantic,
            DiagnosticCode::ResourceLimit,
            span,
            "Reference ID counter overflow",
        )
    })?;
    Ok(ReferenceId(format!("{prefix}-{:0width$}", *counter)))
}

fn visit(blocks: &mut [Located<BlockNode>], counters: &mut IdCounters) -> Result<(), Diagnostic> {
    for block in blocks {
        match &mut block.value {
            BlockNode::Heading(heading) => match heading.level {
                HeadingLevel::H1 => {
                    heading.id = None;
                    counters.parent = None;
                    counters.child = 0;
                }
                HeadingLevel::H2 => {
                    heading.id = Some(next(&mut counters.section, "section", 2, block.span)?);
                    counters.parent = Some(counters.section);
                    counters.child = 0;
                }
                HeadingLevel::H3 => {
                    heading.id = Some(if let Some(parent) = counters.parent {
                        next(
                            &mut counters.child,
                            &format!("section-{parent:02}"),
                            2,
                            block.span,
                        )?
                    } else {
                        next(&mut counters.section, "section", 2, block.span)?
                    });
                }
            },
            BlockNode::Callout(callout) => {
                callout.id = next(&mut counters.callout, "callout", 3, block.span)?;
                visit(&mut callout.blocks, counters)?;
            }
            BlockNode::CodeBlock(code) => {
                code.id = next(&mut counters.code, "code-block", 3, block.span)?;
            }
            BlockNode::CodeGroup(group) => {
                for entry in &mut group.entries {
                    entry.value.code.id = next(&mut counters.code, "code-block", 3, entry.span)?;
                }
            }
            BlockNode::Table(table) => {
                table.id = next(&mut counters.table, "table", 3, block.span)?;
            }
            BlockNode::List(list) => {
                for item in &mut list.items {
                    visit(&mut item.value.blocks, counters)?;
                }
            }
            BlockNode::Quote(quote) => visit(&mut quote.blocks, counters)?,
            BlockNode::Svg(svg) => {
                svg.id = next(&mut counters.svg, "svg", 3, block.span)?;
            }
            BlockNode::Paragraph(_)
            | BlockNode::ThematicBreak
            | BlockNode::Image(_)
            | BlockNode::Video(_)
            | BlockNode::UrlCard(_)
            | BlockNode::TweetCard(_) => {}
        }
    }
    Ok(())
}
