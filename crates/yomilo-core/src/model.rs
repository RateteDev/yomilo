use crate::SourceSpan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyntaxVersion(u16);
impl SyntaxVersion {
    pub const V0: Self = Self(0);
    pub fn number(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelVersion(u16);
impl ModelVersion {
    pub const V0: Self = Self(0);
    pub fn number(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(type = "T", bound = "T: ts_rs::TS"))]
pub struct Located<T> {
    pub value: T,
    #[serde(skip)]
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct ReferenceId(pub(crate) String);
impl ReferenceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredDocument {
    pub(crate) syntax_version: SyntaxVersion,
    pub(crate) model_version: ModelVersion,
    pub(crate) frontmatter: Option<Located<Frontmatter>>,
    pub(crate) blocks: Vec<Located<BlockNode>>,
}

impl StructuredDocument {
    pub fn syntax_version(&self) -> SyntaxVersion {
        self.syntax_version
    }
    pub fn model_version(&self) -> ModelVersion {
        self.model_version
    }
    pub fn frontmatter(&self) -> Option<&Located<Frontmatter>> {
        self.frontmatter.as_ref()
    }
    pub fn blocks(&self) -> &[Located<BlockNode>] {
        &self.blocks
    }

    pub fn same_meaning(&self, other: &Self) -> bool {
        self.syntax_version == other.syntax_version
            && self.model_version == other.model_version
            && self.frontmatter.as_ref().map(|v| &v.value)
                == other.frontmatter.as_ref().map(|v| &v.value)
            && blocks_equal(&self.blocks, &other.blocks)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum FrontmatterFormat {
    Yaml,
    Toml,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Frontmatter {
    pub format: FrontmatterFormat,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum BlockNode {
    Heading(Heading),
    Paragraph(Paragraph),
    List(List),
    Quote(Quote),
    #[serde(rename = "code")]
    CodeBlock(CodeBlock),
    Table(Table),
    ThematicBreak,
    Callout(Callout),
    CodeGroup(CodeGroup),
    Svg(Svg),
    Image(Image),
    Video(Video),
    UrlCard(UrlCard),
    TweetCard(TweetCard),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum InlineNode {
    Text(String),
    Emphasis(Vec<Located<InlineNode>>),
    Strong(Vec<Located<InlineNode>>),
    Code(String),
    Link(Link),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
}
impl HeadingLevel {
    pub fn depth(self) -> u8 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H3 => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum FoldIntent {
    None,
    InitiallyClosed,
    InitiallyOpen,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Heading {
    pub level: HeadingLevel,
    pub fold: FoldIntent,
    pub content: Vec<Located<InlineNode>>,
    pub id: Option<ReferenceId>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Paragraph {
    pub content: Vec<Located<InlineNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Link {
    pub content: Vec<Located<InlineNode>>,
    pub href: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum ListKind {
    Unordered,
    Ordered,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct List {
    pub kind: ListKind,
    pub spread: bool,
    pub items: Vec<Located<ListItem>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct ListItem {
    pub spread: bool,
    pub blocks: Vec<Located<BlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Quote {
    pub blocks: Vec<Located<BlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CodeBlock {
    pub id: ReferenceId,
    pub language: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum ColumnAlignment {
    Unspecified,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Table {
    pub id: ReferenceId,
    pub align: Vec<ColumnAlignment>,
    pub header: Located<TableRow>,
    pub rows: Vec<Located<TableRow>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TableRow {
    pub cells: Vec<Located<TableCell>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TableCell {
    pub content: Vec<Located<InlineNode>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum CalloutType {
    Note,
    Warning,
    Caution,
}
impl CalloutType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Warning => "warning",
            Self::Caution => "caution",
        }
    }
    pub fn default_label(self) -> &'static str {
        match self {
            Self::Note => "NOTE",
            Self::Warning => "WARNING",
            Self::Caution => "CAUTION",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Callout {
    pub id: ReferenceId,
    pub kind: CalloutType,
    pub label: String,
    pub blocks: Vec<Located<BlockNode>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CodeGroup {
    pub entries: Vec<Located<CodeGroupEntry>>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CodeGroupEntry {
    pub label: String,
    pub code: CodeBlock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Ratio {
    pub(crate) width: u32,
    pub(crate) height: u32,
}
impl Ratio {
    pub fn width(self) -> u32 {
        self.width
    }
    pub fn height(self) -> u32 {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Image {
    pub src: String,
    pub alt: Option<String>,
    pub ratio: Option<Ratio>,
    pub caption: Option<String>,
    pub zoom: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Video {
    pub src: String,
    pub ratio: Option<Ratio>,
    pub caption: Option<String>,
    pub loop_playback: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct ValidatedSvg {
    pub(crate) source: String,
    #[serde(rename = "elementNames")]
    pub(crate) elements: Vec<String>,
    #[serde(rename = "attributeNames")]
    pub(crate) attributes: Vec<String>,
}
impl ValidatedSvg {
    pub fn as_str(&self) -> &str {
        &self.source
    }

    pub fn element_names(&self) -> &[String] {
        &self.elements
    }

    pub fn attribute_names(&self) -> &[String] {
        // SMILの対象属性とanimateMotionが変更するtransformも含む。
        &self.attributes
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct Svg {
    pub id: ReferenceId,
    pub body: ValidatedSvg,
    pub alt: Option<String>,
    pub caption: Option<String>,
    pub zoom: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct UrlCard {
    pub href: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TweetCard {
    pub href: String,
}

fn slices_equal<T>(a: &[Located<T>], b: &[Located<T>], eq: impl Fn(&T, &T) -> bool) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| eq(&a.value, &b.value))
}

fn inlines_equal(a: &[Located<InlineNode>], b: &[Located<InlineNode>]) -> bool {
    slices_equal(a, b, |a, b| match (a, b) {
        (InlineNode::Text(a), InlineNode::Text(b)) | (InlineNode::Code(a), InlineNode::Code(b)) => {
            a == b
        }
        (InlineNode::Emphasis(a), InlineNode::Emphasis(b))
        | (InlineNode::Strong(a), InlineNode::Strong(b)) => inlines_equal(a, b),
        (InlineNode::Link(a), InlineNode::Link(b)) => {
            a.href == b.href && inlines_equal(&a.content, &b.content)
        }
        _ => false,
    })
}

fn rows_equal(a: &TableRow, b: &TableRow) -> bool {
    slices_equal(&a.cells, &b.cells, |a, b| {
        inlines_equal(&a.content, &b.content)
    })
}

fn blocks_equal(a: &[Located<BlockNode>], b: &[Located<BlockNode>]) -> bool {
    slices_equal(a, b, |a, b| match (a, b) {
        (BlockNode::Heading(a), BlockNode::Heading(b)) => {
            a.level == b.level
                && a.fold == b.fold
                && a.id == b.id
                && inlines_equal(&a.content, &b.content)
        }
        (BlockNode::Paragraph(a), BlockNode::Paragraph(b)) => inlines_equal(&a.content, &b.content),
        (BlockNode::List(a), BlockNode::List(b)) => {
            a.kind == b.kind
                && a.spread == b.spread
                && slices_equal(&a.items, &b.items, |a, b| {
                    a.spread == b.spread && blocks_equal(&a.blocks, &b.blocks)
                })
        }
        (BlockNode::Quote(a), BlockNode::Quote(b)) => blocks_equal(&a.blocks, &b.blocks),
        (BlockNode::CodeBlock(a), BlockNode::CodeBlock(b)) => a == b,
        (BlockNode::Table(a), BlockNode::Table(b)) => {
            a.id == b.id
                && a.align == b.align
                && rows_equal(&a.header.value, &b.header.value)
                && slices_equal(&a.rows, &b.rows, rows_equal)
        }
        (BlockNode::ThematicBreak, BlockNode::ThematicBreak) => true,
        (BlockNode::Callout(a), BlockNode::Callout(b)) => {
            a.id == b.id
                && a.kind == b.kind
                && a.label == b.label
                && blocks_equal(&a.blocks, &b.blocks)
        }
        (BlockNode::CodeGroup(a), BlockNode::CodeGroup(b)) => {
            slices_equal(&a.entries, &b.entries, |a, b| a == b)
        }
        (BlockNode::Svg(a), BlockNode::Svg(b)) => a == b,
        (BlockNode::Image(a), BlockNode::Image(b)) => a == b,
        (BlockNode::Video(a), BlockNode::Video(b)) => a == b,
        (BlockNode::UrlCard(a), BlockNode::UrlCard(b)) => a == b,
        (BlockNode::TweetCard(a), BlockNode::TweetCard(b)) => a == b,
        _ => false,
    })
}
