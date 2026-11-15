use crate::{
    CalloutType, Frontmatter, HeadingLevel, InlineNode, ListKind, Located, ModelVersion, Paragraph,
    Ratio, ReferenceId, RejectedSegment, SourceSpan, Svg, SyntaxVersion, Table,
};
use std::{collections::BTreeMap, error::Error, fmt};

pub const PREPARATION_VERSION: u16 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct PreparationVersions {
    pub syntax_version: u16,
    pub model_version: u16,
    pub preparation_version: u16,
}

impl PreparationVersions {
    pub fn current() -> Self {
        Self {
            syntax_version: SyntaxVersion::V0.number(),
            model_version: ModelVersion::V0.number(),
            preparation_version: PREPARATION_VERSION,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct PreparationInputs {
    pub versions: PreparationVersions,
    #[serde(default)]
    pub url_cards: BTreeMap<String, UrlCardFetch>,
    #[serde(default)]
    pub tweet_cards: BTreeMap<String, TweetState>,
    #[serde(default)]
    pub images: BTreeMap<String, MediaDelivery>,
    #[serde(default)]
    pub videos: BTreeMap<String, MediaDelivery>,
}

impl Default for PreparationInputs {
    fn default() -> Self {
        Self {
            versions: PreparationVersions::current(),
            url_cards: BTreeMap::new(),
            tweet_cards: BTreeMap::new(),
            images: BTreeMap::new(),
            videos: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "state",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum FetchState<T> {
    Available(T),
    Missing,
    Failed {
        message: String,
    },
    #[default]
    NotRequested,
}

impl<T> FetchState<T> {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available(_) => "available",
            Self::Missing => "missing",
            Self::Failed { .. } => "failed",
            Self::NotRequested => "not-requested",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct UrlCardFetch {
    pub title: FetchState<String>,
    pub icon: FetchState<MediaDelivery>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum MediaDelivery {
    Reference {
        reference: String,
    },
    Embedded {
        media_type: String,
        #[serde(with = "crate::wire::base64_bytes")]
        #[cfg_attr(feature = "typescript", ts(type = "string"))]
        bytes: Vec<u8>,
    },
}

impl MediaDelivery {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reference { .. } => "reference",
            Self::Embedded { .. } => "embedded",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum MediaKind {
    Image,
    Video,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "state",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum TweetState {
    Available(Box<TweetData>),
    Unavailable {
        reason: TweetUnavailableReason,
    },
    #[default]
    NotRequested,
}

impl TweetState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available(_) => "available",
            Self::Unavailable { .. } => "unavailable",
            Self::NotRequested => "not-requested",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum TweetUnavailableReason {
    Missing,
    Failed { message: String },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum TweetDateTime {
    Original(String),
    UnixSeconds(i64),
    UnixMilliseconds(i64),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetData {
    pub id: String,
    pub author: TweetAuthor,
    pub text: String,
    pub created_at: TweetDateTime,
    pub counts: TweetCounts,
    pub links: Vec<TweetLink>,
    pub media: Vec<TweetMedia>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetAuthor {
    pub name: String,
    pub handle: String,
    pub profile_href: String,
    pub avatar: Option<MediaDelivery>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetCounts {
    pub replies: Option<u64>,
    pub reposts: Option<u64>,
    pub likes: Option<u64>,
    pub quotes: Option<u64>,
    pub views: Option<u64>,
    pub bookmarks: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetLink {
    // アダプタが元本文に対するUTF-8バイト範囲へ変換する。
    pub start: usize,
    pub end: usize,
    pub href: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetMedia {
    pub kind: MediaKind,
    pub delivery: MediaDelivery,
    pub alt: Option<String>,
    pub dimensions: Option<MediaDimensions>,
    pub poster: Option<MediaDelivery>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct MediaDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderSegment {
    #[serde(deserialize_with = "crate::wire::deserialize_versions")]
    pub versions: PreparationVersions,
    pub index: usize,
    pub content: RenderSegmentContent,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum RenderSegmentContent {
    Frontmatter(Located<Frontmatter>),
    Block(Located<RenderBlockNode>),
    Rejected(RejectedSegment),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum SectionFold {
    None,
    Open,
    Closed,
}

impl SectionFold {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderSectionHeading {
    pub id: ReferenceId,
    pub level: HeadingLevel,
    pub fold: SectionFold,
    pub content: Vec<Located<InlineNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderTitle {
    pub fold: SectionFold,
    pub content: Vec<Located<InlineNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum RenderBlockNode {
    Title(RenderTitle),
    SectionHeading(RenderSectionHeading),
    Paragraph(Paragraph),
    List(RenderList),
    Quote(RenderQuote),
    #[serde(rename = "code")]
    CodeBlock(RenderCodeBlock),
    Table(Table),
    ThematicBreak,
    Callout(RenderCallout),
    CodeGroup(RenderCodeGroup),
    Svg(Svg),
    Image(RenderImage),
    Video(RenderVideo),
    UrlCard(RenderUrlCard),
    TweetCard(RenderTweetCard),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderList {
    pub kind: ListKind,
    pub spread: bool,
    pub items: Vec<Located<RenderListItem>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderListItem {
    pub spread: bool,
    pub blocks: Vec<Located<RenderBlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderQuote {
    pub blocks: Vec<Located<RenderBlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderCallout {
    pub id: ReferenceId,
    pub kind: CalloutType,
    pub label: String,
    pub blocks: Vec<Located<RenderBlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderCodeBlock {
    pub id: ReferenceId,
    #[serde(rename = "sourceLanguage")]
    pub language: String,
    pub value: String,
    pub highlighting: CodeHighlighting,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum CodeHighlighting {
    Highlighted {
        language: super::CodeLanguage,
        tokens: Vec<CodeToken>,
    },
    Plain {
        reason: PlainCodeReason,
    },
}

impl CodeHighlighting {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Highlighted { .. } => "highlighted",
            Self::Plain { .. } => "plain",
        }
    }
    pub fn tokens(&self) -> &[CodeToken] {
        match self {
            Self::Highlighted { tokens, .. } => tokens,
            Self::Plain { .. } => &[],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum PlainCodeReason {
    NoLanguage,
    ExplicitPlain,
    UnknownLanguage,
    TokenizationFailed { message: String },
}

impl PlainCodeReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoLanguage => "no-language",
            Self::ExplicitPlain => "explicit-plain",
            Self::UnknownLanguage => "unknown-language",
            Self::TokenizationFailed { .. } => "tokenization-failed",
        }
    }
}

// TODO(role-detail): 将来はstring.double等の第2段を追加し、表示側が最長一致で上書きできる型へ拡張する。
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum CodeRole {
    Comment,
    Keyword,
    String,
    Function,
    Type,
    Property,
    Variable,
    Punctuation,
}

impl CodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Keyword => "keyword",
            Self::String => "string",
            Self::Function => "function",
            Self::Type => "type",
            Self::Property => "property",
            Self::Variable => "variable",
            Self::Punctuation => "punctuation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CodeToken {
    pub start: usize,
    pub end: usize,
    pub role: CodeRole,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderCodeGroup {
    pub entries: Vec<Located<RenderCodeGroupEntry>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderCodeGroupEntry {
    pub index: usize,
    pub label: String,
    pub code: RenderCodeBlock,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderImage {
    pub src: String,
    pub delivery: MediaDelivery,
    pub alt: Option<String>,
    pub ratio: Option<Ratio>,
    pub caption: Option<String>,
    pub zoom: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderVideo {
    pub src: String,
    pub delivery: MediaDelivery,
    pub ratio: Option<Ratio>,
    pub caption: Option<String>,
    pub loop_playback: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderUrlCard {
    pub href: String,
    pub title: FetchState<String>,
    pub icon: FetchState<MediaDelivery>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct RenderTweetCard {
    pub href: String,
    pub state: TweetState,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum PreparationError {
    VersionMismatch {
        expected: PreparationVersions,
        actual: PreparationVersions,
    },
    MissingMediaInput {
        kind: MediaKind,
        reference: String,
        span: SourceSpan,
    },
    InvalidInput {
        message: String,
        span: SourceSpan,
    },
    InvalidDocument {
        message: String,
        span: SourceSpan,
    },
    MissingSyntax {
        syntax_name: String,
    },
}

impl fmt::Display for PreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionMismatch { expected, actual } => write!(
                f,
                "Preparation version mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::MissingMediaInput {
                kind, reference, ..
            } => write!(
                f,
                "Missing {} delivery input for {reference:?}",
                kind.as_str()
            ),
            Self::InvalidInput { message, .. } => write!(f, "Invalid preparation input: {message}"),
            Self::InvalidDocument { message, .. } => {
                write!(f, "Invalid structured document: {message}")
            }
            Self::MissingSyntax { syntax_name } => write!(
                f,
                "Selected code syntax {syntax_name:?} is missing from the embedded syntax set"
            ),
        }
    }
}

impl Error for PreparationError {}
