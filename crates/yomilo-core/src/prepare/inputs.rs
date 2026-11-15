use super::model::{
    MediaDelivery, MediaDimensions, MediaKind, TweetAuthor, TweetCounts, TweetData, TweetDateTime,
    TweetMedia, TweetState, TweetUnavailableReason,
};
use crate::{BlockNode, Located, Segment, SegmentContent};
use regex::Regex;
use serde_json::Value;
use std::{collections::BTreeSet, sync::LazyLock};

/// 描画準備に供給すべき入力の一覧。どれも原稿に書かれた名前とURLで、重複を除き昇順に並ぶ。
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RequiredInputs {
    pub images: Vec<String>,
    pub videos: Vec<String>,
    pub url_cards: Vec<String>,
    pub tweet_cards: Vec<String>,
}

#[derive(Default)]
struct Collected {
    images: BTreeSet<String>,
    videos: BTreeSet<String>,
    url_cards: BTreeSet<String>,
    tweet_cards: BTreeSet<String>,
}

pub fn required_inputs(segment: &Segment) -> RequiredInputs {
    let mut collected = Collected::default();
    if let SegmentContent::Block(block) = &segment.content {
        collect(std::slice::from_ref(block), &mut collected);
    }
    RequiredInputs {
        images: collected.images.into_iter().collect(),
        videos: collected.videos.into_iter().collect(),
        url_cards: collected.url_cards.into_iter().collect(),
        tweet_cards: collected.tweet_cards.into_iter().collect(),
    }
}

fn collect(blocks: &[Located<BlockNode>], collected: &mut Collected) {
    for block in blocks {
        match &block.value {
            BlockNode::Image(image) => {
                collected.images.insert(image.src.clone());
            }
            BlockNode::Video(video) => {
                collected.videos.insert(video.src.clone());
            }
            BlockNode::UrlCard(card) => {
                collected.url_cards.insert(card.href.clone());
            }
            BlockNode::TweetCard(card) => {
                collected.tweet_cards.insert(card.href.clone());
            }
            BlockNode::List(list) => {
                for item in &list.items {
                    collect(&item.value.blocks, collected);
                }
            }
            BlockNode::Quote(quote) => collect(&quote.blocks, collected),
            BlockNode::Callout(callout) => collect(&callout.blocks, collected),
            BlockNode::Heading(_)
            | BlockNode::Paragraph(_)
            | BlockNode::CodeBlock(_)
            | BlockNode::Table(_)
            | BlockNode::ThematicBreak
            | BlockNode::CodeGroup(_)
            | BlockNode::Svg(_) => {}
        }
    }
}

fn pattern(source: &str) -> Regex {
    Regex::new(source).expect("static pattern")
}

static ENTITY: LazyLock<Regex> = LazyLock::new(|| pattern(r"(?i)&(#x[0-9a-f]+|#[0-9]+|[a-z]+);"));
static OG_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r#"(?i)<meta[^>]+(?:property|name)=["']og:title["'][^>]+content=["']([^"']*?)["']"#)
});
static OG_TITLE_CONTENT_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r#"(?i)<meta[^>]+content=["']([^"']*?)["'][^>]+(?:property|name)=["']og:title["']"#)
});
static TITLE: LazyLock<Regex> = LazyLock::new(|| pattern(r"(?i)<title[^>]*>([^<]*)</title>"));

/// 正規表現で抜き出したHTMLの値に残る文字参照を戻す。戻さないとHTML描画が二重にエスケープする。
fn decode_entities(text: &str) -> String {
    ENTITY
        .replace_all(text, |captures: &regex::Captures<'_>| {
            let whole = &captures[0];
            let body = &captures[1];
            let decoded = if let Some(number) = body.strip_prefix('#') {
                let code = match number.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse::<u32>().ok(),
                };
                code.filter(|code| *code > 0).and_then(char::from_u32)
            } else {
                match body.to_ascii_lowercase().as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some('\u{a0}'),
                    _ => None,
                }
            };
            decoded.map_or_else(|| whole.to_owned(), String::from)
        })
        .into_owned()
}

/// 取得したページのHTMLからURLカードの題名を読む。`og:title`を優先し、無ければ`<title>`を使う。
pub fn interpret_url_card_page(html: &str) -> Option<String> {
    if let Some(captures) = OG_TITLE
        .captures(html)
        .or_else(|| OG_TITLE_CONTENT_FIRST.captures(html))
    {
        return Some(decode_entities(&captures[1]));
    }
    let title = TITLE.captures(html)?.get(1)?.as_str().trim();
    (!title.is_empty()).then(|| decode_entities(title))
}

/// Xの`tweet-result`の応答を読む。画像は参照で返し、埋め込むかは利用側が差し替えて決める。
pub fn interpret_tweet_result(id: &str, json: &str) -> TweetState {
    let failed = |message: String| TweetState::Unavailable {
        reason: TweetUnavailableReason::Failed { message },
    };
    let root: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(error) => return failed(format!("Invalid tweet result JSON: {error}")),
    };
    let str_of =
        |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let Some(user) = root.get("user").filter(|user| user.is_object()) else {
        return TweetState::Unavailable {
            reason: TweetUnavailableReason::Missing,
        };
    };
    if root.get("__typename").and_then(Value::as_str) == Some("TweetTombstone") {
        return TweetState::Unavailable {
            reason: TweetUnavailableReason::Missing,
        };
    }
    let (Some(name), Some(handle)) = (str_of(user, "name"), str_of(user, "screen_name")) else {
        return failed("Tweet result has no author name or handle".to_owned());
    };
    let Some(created_at) = str_of(&root, "created_at") else {
        return failed("Tweet result has no created_at".to_owned());
    };
    let reference = |url: String| MediaDelivery::Reference { reference: url };
    let count = |key: &str| Some(root.get(key).and_then(Value::as_u64).unwrap_or(0));
    let media = root
        .get("mediaDetails")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let url = str_of(item, "media_url_https")?;
            let size = |key: &str| {
                item.get("original_info")
                    .and_then(|info| info.get(key))
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .unwrap_or(0)
            };
            let (width, height) = (size("width"), size("height"));
            Some(TweetMedia {
                // 動画とGIFの`media_url_https`もプレビュー画像。動画として偽らない。
                kind: MediaKind::Image,
                delivery: reference(url),
                alt: str_of(item, "ext_alt_text"),
                dimensions: (width > 0 && height > 0).then_some(MediaDimensions { width, height }),
                poster: None,
            })
        })
        .collect();
    TweetState::Available(Box::new(TweetData {
        id: str_of(&root, "id_str").unwrap_or_else(|| id.to_owned()),
        author: TweetAuthor {
            profile_href: format!("https://x.com/{handle}"),
            avatar: str_of(user, "profile_image_url_https").map(reference),
            name,
            handle,
        },
        text: tweet_text(&root),
        created_at: TweetDateTime::Original(created_at),
        counts: TweetCounts {
            replies: count("conversation_count"),
            likes: count("favorite_count"),
            ..TweetCounts::default()
        },
        // TODO(engine-tweet-fields): 本文中のリンク範囲、認証、返信先、引用投稿は未対応。応答のentitiesから組み立てる。
        links: Vec::new(),
        media,
    }))
}

/// 短縮URLを表示用のURLへ戻し、添付を指す短縮URLは本文から外す。
fn tweet_text(root: &Value) -> String {
    let mut text = decode_entities(root.get("text").and_then(Value::as_str).unwrap_or_default());
    let entities = |key: &str| {
        root.get("entities")
            .and_then(|entities| entities.get(key))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for url in entities("urls") {
        if let (Some(short), Some(display)) = (
            url.get("url").and_then(Value::as_str),
            url.get("display_url").and_then(Value::as_str),
        ) {
            text = text.replacen(short, display, 1);
        }
    }
    for media in entities("media") {
        if let Some(short) = media.get("url").and_then(Value::as_str) {
            text = text.replacen(short, "", 1);
        }
    }
    text.trim().to_owned()
}
