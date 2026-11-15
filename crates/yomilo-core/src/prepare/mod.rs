mod code;
mod inputs;
mod model;

pub use code::{
    CodeLanguage, CodeLanguageDefinition, PLAIN_LANGUAGE_NAMES, code_languages,
    recognize_code_language,
};
pub use inputs::{
    RequiredInputs, interpret_tweet_result, interpret_url_card_page, required_inputs,
};
pub use model::*;

use crate::{
    BlockNode, CodeGroup, FoldIntent, HeadingLevel, Located, Segment, SegmentContent, SourceSpan,
};
use std::collections::BTreeSet;

/// 確定したセグメントを1つ描画準備する。前後のセグメントには依存しないため、どの順に準備してもよい。
pub fn prepare_segment(
    segment: &Segment,
    inputs: &PreparationInputs,
) -> Result<RenderSegment, PreparationError> {
    let expected = PreparationVersions::current();
    let actual = PreparationVersions {
        syntax_version: segment.versions.syntax_version,
        model_version: segment.versions.model_version,
        preparation_version: PREPARATION_VERSION,
    };
    check_versions(expected, actual)?;
    check_versions(expected, inputs.versions)?;
    let content = match &segment.content {
        SegmentContent::Frontmatter(frontmatter) => {
            RenderSegmentContent::Frontmatter(frontmatter.clone())
        }
        SegmentContent::Block(block) => RenderSegmentContent::Block(Located {
            value: Builder { inputs }.block(&block.value, segment.span)?,
            span: block.span,
        }),
        SegmentContent::Rejected(rejected) => RenderSegmentContent::Rejected(rejected.clone()),
    };
    Ok(RenderSegment {
        versions: actual,
        index: segment.index,
        content,
    })
}

pub fn check_preparation_versions(actual: PreparationVersions) -> Result<(), PreparationError> {
    check_versions(PreparationVersions::current(), actual)
}

// TODO(version-freeze): 版が変わったときに公開済みの記事をどう保つか（古いエンジンを残すか、出力を固定するか）は未決定。
fn check_versions(
    expected: PreparationVersions,
    actual: PreparationVersions,
) -> Result<(), PreparationError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PreparationError::VersionMismatch { expected, actual })
    }
}

struct Builder<'a> {
    inputs: &'a PreparationInputs,
}

impl Builder<'_> {
    fn blocks(
        &mut self,
        source: &[Located<BlockNode>],
    ) -> Result<Vec<Located<RenderBlockNode>>, PreparationError> {
        source
            .iter()
            .map(|block| {
                Ok(Located {
                    value: self.block(&block.value, block.span)?,
                    span: block.span,
                })
            })
            .collect()
    }

    fn block(
        &mut self,
        block: &BlockNode,
        span: SourceSpan,
    ) -> Result<RenderBlockNode, PreparationError> {
        Ok(match block {
            BlockNode::Heading(heading) => {
                let fold = match heading.fold {
                    FoldIntent::None => SectionFold::None,
                    FoldIntent::InitiallyOpen => SectionFold::Open,
                    FoldIntent::InitiallyClosed => SectionFold::Closed,
                };
                if heading.level == HeadingLevel::H1 {
                    RenderBlockNode::Title(RenderTitle {
                        fold,
                        content: heading.content.clone(),
                    })
                } else {
                    let id =
                        heading
                            .id
                            .clone()
                            .ok_or_else(|| PreparationError::InvalidDocument {
                                message: "A section heading must have a Core-assigned ID"
                                    .to_owned(),
                                span,
                            })?;
                    RenderBlockNode::SectionHeading(RenderSectionHeading {
                        id,
                        level: heading.level,
                        fold,
                        content: heading.content.clone(),
                    })
                }
            }
            BlockNode::Paragraph(paragraph) => RenderBlockNode::Paragraph(paragraph.clone()),
            BlockNode::List(list) => {
                let items = list
                    .items
                    .iter()
                    .map(|item| {
                        Ok(Located {
                            value: RenderListItem {
                                spread: item.value.spread,
                                blocks: self.blocks(&item.value.blocks)?,
                            },
                            span: item.span,
                        })
                    })
                    .collect::<Result<_, PreparationError>>()?;
                RenderBlockNode::List(RenderList {
                    kind: list.kind,
                    spread: list.spread,
                    items,
                })
            }
            BlockNode::Quote(quote) => RenderBlockNode::Quote(RenderQuote {
                blocks: self.blocks(&quote.blocks)?,
            }),
            BlockNode::CodeBlock(code) => RenderBlockNode::CodeBlock(code::prepare_code(code)?),
            BlockNode::Table(table) => RenderBlockNode::Table(table.clone()),
            BlockNode::ThematicBreak => RenderBlockNode::ThematicBreak,
            BlockNode::Callout(callout) => RenderBlockNode::Callout(RenderCallout {
                id: callout.id.clone(),
                kind: callout.kind,
                label: callout.label.clone(),
                blocks: self.blocks(&callout.blocks)?,
            }),
            BlockNode::CodeGroup(group) => {
                RenderBlockNode::CodeGroup(self.code_group(group, span)?)
            }
            BlockNode::Svg(svg) => {
                let mut svg = svg.clone();
                crate::svg::namespace(&mut svg.body, &format!("yomilo-{}", svg.id.as_str()), span)
                    .map_err(|diagnostic| PreparationError::InvalidDocument {
                        message: diagnostic.message,
                        span: diagnostic.span,
                    })?;
                RenderBlockNode::Svg(svg)
            }
            BlockNode::Image(image) => RenderBlockNode::Image(RenderImage {
                src: image.src.clone(),
                delivery: self.media(MediaKind::Image, &image.src, span)?,
                alt: image.alt.clone(),
                ratio: image.ratio,
                caption: image.caption.clone(),
                zoom: image.zoom,
            }),
            BlockNode::Video(video) => RenderBlockNode::Video(RenderVideo {
                src: video.src.clone(),
                delivery: self.media(MediaKind::Video, &video.src, span)?,
                ratio: video.ratio,
                caption: video.caption.clone(),
                loop_playback: video.loop_playback,
            }),
            BlockNode::UrlCard(card) => {
                let fetch = self
                    .inputs
                    .url_cards
                    .get(&card.href)
                    .cloned()
                    .unwrap_or_default();
                if let FetchState::Available(delivery) = &fetch.icon {
                    validate_delivery(delivery, MediaKind::Image, span)?;
                }
                RenderBlockNode::UrlCard(RenderUrlCard {
                    href: card.href.clone(),
                    title: fetch.title,
                    icon: fetch.icon,
                })
            }
            BlockNode::TweetCard(card) => {
                let state = self
                    .inputs
                    .tweet_cards
                    .get(&card.href)
                    .cloned()
                    .unwrap_or_default();
                if let TweetState::Available(tweet) = &state {
                    validate_tweet(tweet, span)?;
                }
                RenderBlockNode::TweetCard(RenderTweetCard {
                    href: card.href.clone(),
                    state,
                })
            }
        })
    }

    fn code_group(
        &self,
        group: &CodeGroup,
        span: SourceSpan,
    ) -> Result<RenderCodeGroup, PreparationError> {
        if group.entries.is_empty() {
            return Err(PreparationError::InvalidDocument {
                message: "A CodeGroup must have at least one labelled code entry".to_owned(),
                span,
            });
        }
        let mut labels = BTreeSet::new();
        let entries = group
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                if entry.value.label.trim().is_empty() || !labels.insert(&entry.value.label) {
                    return Err(PreparationError::InvalidDocument {
                        message: "CodeGroup labels must be nonempty and unique".to_owned(),
                        span: entry.span,
                    });
                }
                Ok(Located {
                    value: RenderCodeGroupEntry {
                        index,
                        label: entry.value.label.clone(),
                        code: code::prepare_code(&entry.value.code)?,
                    },
                    span: entry.span,
                })
            })
            .collect::<Result<_, PreparationError>>()?;
        Ok(RenderCodeGroup { entries })
    }

    fn media(
        &self,
        kind: MediaKind,
        reference: &str,
        span: SourceSpan,
    ) -> Result<MediaDelivery, PreparationError> {
        let inputs = match kind {
            MediaKind::Image => &self.inputs.images,
            MediaKind::Video => &self.inputs.videos,
        };
        let delivery =
            inputs
                .get(reference)
                .ok_or_else(|| PreparationError::MissingMediaInput {
                    kind,
                    reference: reference.to_owned(),
                    span,
                })?;
        validate_delivery(delivery, kind, span)?;
        Ok(delivery.clone())
    }
}

fn validate_delivery(
    delivery: &MediaDelivery,
    kind: MediaKind,
    span: SourceSpan,
) -> Result<(), PreparationError> {
    match delivery {
        MediaDelivery::Reference { reference } => safe_reference(reference, false, span),
        MediaDelivery::Embedded { media_type, .. } => {
            let essence = media_type.split(';').next().expect("MIME essence").trim();
            let (major, minor) =
                essence
                    .split_once('/')
                    .ok_or_else(|| PreparationError::InvalidInput {
                        message: format!("Invalid media type {media_type:?}"),
                        span,
                    })?;
            if !major.eq_ignore_ascii_case(kind.as_str())
                || minor.is_empty()
                || !minor
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&ch))
                || !media_type
                    .bytes()
                    .all(|ch| ch.is_ascii() && (!ch.is_ascii_control() || ch == b'\t'))
            {
                return Err(PreparationError::InvalidInput {
                    message: format!(
                        "Expected an {} MIME type, got {media_type:?}",
                        kind.as_str()
                    ),
                    span,
                });
            }
            Ok(())
        }
    }
}

fn safe_reference(
    reference: &str,
    absolute: bool,
    span: SourceSpan,
) -> Result<(), PreparationError> {
    crate::validate::reference(reference, span, false, absolute).map_err(|diagnostic| {
        PreparationError::InvalidInput {
            message: format!(
                "Unsafe delivery or metadata reference {reference:?}: {}",
                diagnostic.message
            ),
            span,
        }
    })
}

fn validate_tweet(tweet: &TweetData, span: SourceSpan) -> Result<(), PreparationError> {
    safe_reference(&tweet.author.profile_href, true, span)?;
    if let Some(avatar) = &tweet.author.avatar {
        validate_delivery(avatar, MediaKind::Image, span)?;
    }
    let mut end = 0;
    for link in &tweet.links {
        if link.start < end
            || link.start >= link.end
            || link.end > tweet.text.len()
            || !tweet.text.is_char_boundary(link.start)
            || !tweet.text.is_char_boundary(link.end)
        {
            return Err(PreparationError::InvalidInput { message: "Tweet links must have ordered, nonoverlapping UTF-8 byte ranges within the original text".to_owned(), span });
        }
        safe_reference(&link.href, true, span)?;
        end = link.end;
    }
    for media in &tweet.media {
        if media
            .dimensions
            .is_some_and(|dimensions| dimensions.width == 0 || dimensions.height == 0)
        {
            return Err(PreparationError::InvalidInput {
                message: "Tweet media dimensions must be positive".to_owned(),
                span,
            });
        }
        validate_delivery(&media.delivery, media.kind, span)?;
        if let Some(poster) = &media.poster {
            validate_delivery(poster, MediaKind::Image, span)?;
        }
    }
    Ok(())
}
