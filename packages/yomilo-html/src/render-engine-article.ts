// hastのrawノードの型拡張は、このファイルから読む。利用側が配布物のTSソースを型検査するとき、
// 利用側のtsconfigからnode_modulesの別名経由で拡張だけを読むと、hastの解決先がずれて拡張が効かないため。
/// <reference path="./hast-raw.d.ts" />
import type { MediaDelivery, RenderBlockNode, RenderSegment, TweetData } from "@yomilo/wasm";
import type { Element, ElementContent, Root, Text } from "hast";
import { toHtml } from "hast-util-to-html";
import { buildCallout } from "./article-blocks.js";
import { rehypeArticleStructure } from "./article-structure.js";
import { normalizeEngineInlineSoftbreaks } from "./engine-inline-softbreaks.js";
import { assertEngineSvgPresentation } from "./engine-svg-presentation.js";
import { createHeadingIdGenerator, extractHeadingText } from "./heading-slug.js";
import { escapeHtml } from "./html-escape.js";
import { buildImage, buildVideo, EAGER_IMAGE_COUNT } from "./media.js";
import { readEngineFrontmatter } from "./render-engine-frontmatter.js";
import { rehypeHeadingPhraseWrap, rehypeTableWrap } from "./render-tree-transforms.js";
import { MOTION_ELEMENTS } from "./svg-motion.js";
import { buildTweetCard } from "./tweet-card-build.js";
import type { Diagnostic, Heading } from "./types.js";
import { buildUrlCard } from "./url-card.js";

type BlockValue<K extends Exclude<RenderBlockNode["kind"], "thematic-break">> = Extract<
  RenderBlockNode,
  { kind: K }
>["value"];
type Inline = BlockValue<"paragraph">["content"][number];
type Code = BlockValue<"code">;
type CodeRole = Extract<
  Code["highlighting"],
  { kind: "highlighted" }
>["value"]["tokens"][number]["role"];

const element = (
  tagName: string,
  properties: Element["properties"] = {},
  children: ElementContent[] = [],
): Element => ({ type: "element", tagName, properties, children });
const text = (value: string): Text => ({ type: "text", value });

function resolveEngineMedia(delivery: MediaDelivery): string {
  const source =
    delivery.kind === "embedded"
      ? `data:${delivery.value.mediaType};base64,${delivery.value.bytes}`
      : delivery.value.reference;
  if (
    !source ||
    (delivery.kind === "embedded" && (!delivery.value.mediaType || !delivery.value.bytes))
  ) {
    throw new Error("Media delivery must contain a non-empty source");
  }
  return source;
}

function inlineNodes(nodes: Inline[]): ElementContent[] {
  const result = buildInlineNodes(nodes);
  normalizeEngineInlineSoftbreaks(result);
  return result;
}

function inlineText(value: string): Text {
  const node = text(value);
  normalizeEngineInlineSoftbreaks([node]);
  return node;
}

function buildInlineNodes(nodes: Inline[]): ElementContent[] {
  return nodes.map((node) => {
    switch (node.kind) {
      case "text":
        return text(node.value);
      case "code":
        return element("code", {}, [text(node.value)]);
      case "emphasis":
      case "strong":
        return element(
          node.kind === "emphasis" ? "em" : "strong",
          {},
          buildInlineNodes(node.value),
        );
      case "link":
        return element("a", { href: node.value.href }, buildInlineNodes(node.value.content));
      default:
        return unreachable(node);
    }
  });
}

function unreachable(value: never): never {
  throw new Error(`Unsupported render node: ${JSON.stringify(value)}`);
}

// 役割はdata-code-roleだけに残し、色はテーマを読む所有CSSへ委ねる。
// 配色の差し替えでRendererやコード本文を変えずに済むようにする。
function codePre(code: Code): Element {
  const lines: ElementContent[][] = [[]];
  const append = (value: string, role?: CodeRole): void => {
    const parts = value.split("\n");
    for (const [index, part] of parts.entries()) {
      if (index > 0) lines.push([]);
      if (part) {
        const line = lines[lines.length - 1];
        if (!line) throw new Error("Code line is missing");
        line.push(role ? element("span", { dataCodeRole: role }, [text(part)]) : text(part));
      }
    }
  };

  if (code.highlighting.kind === "highlighted") {
    // start/endはUTF-16の文字位置ではなく、UTF-8のバイト位置である。
    const bytes = Buffer.from(code.value, "utf8");
    const decoder = new TextDecoder("utf-8", { fatal: true });
    let offset = 0;
    for (const token of code.highlighting.value.tokens) {
      if (
        !Number.isInteger(token.start) ||
        !Number.isInteger(token.end) ||
        token.start < offset ||
        token.end < token.start ||
        token.end > bytes.length
      ) {
        throw new Error(`Invalid UTF-8 code token range in ${code.id}`);
      }
      append(decoder.decode(bytes.subarray(offset, token.start)));
      append(decoder.decode(bytes.subarray(token.start, token.end)), token.role);
      offset = token.end;
    }
    append(decoder.decode(bytes.subarray(offset)));
  } else {
    append(code.value);
  }

  const content: ElementContent[] = [];
  for (const [index, line] of lines.entries()) {
    if (index > 0) content.push(text("\n"));
    content.push(element("span", { className: ["line"] }, line));
  }
  return element(
    "pre",
    {
      dataLanguage: code.sourceLanguage,
      className: ["engine-code"],
      tabIndex: 0,
    },
    [element("code", {}, content)],
  );
}

// 本文だけの出力では、言語名とCodeGroupのラベルを読める文字として残し、操作子を足さない。
function codeBlock(code: Code, label?: string): Element {
  return element("div", { className: ["code-block-wrapper"], dataLanguage: code.sourceLanguage }, [
    element("div", { className: ["code-block-header"] }, [
      ...(label === undefined
        ? []
        : [element("span", { className: ["code-group-label"] }, [text(label)])]),
      element("span", { className: ["code-lang"] }, [text(code.sourceLanguage)]),
    ]),
    codePre(code),
  ]);
}

function svgFigure(svg: BlockValue<"svg">): Element {
  assertEngineSvgPresentation(svg.body.source);
  const motion = svg.body.elementNames.some((name) => MOTION_ELEMENTS.has(name));
  // エンジンが処理済みの内部idと参照を保持する。SVGを再接頭辞化しない。
  const source = svg.alt
    ? svg.body.source.replace(
        /<svg(?=[\s>])/,
        `<svg role="img" aria-label="${escapeHtml(svg.alt)}"`,
      )
    : svg.body.source;
  const stage = element("div", { className: ["svg-figure-stage"] }, [
    { type: "raw", value: source },
  ]);
  return element(
    "figure",
    {
      className: ["media", "svg-figure"],
      ...(motion ? { dataMotion: "" } : {}),
      ...(svg.zoom ? { dataZoom: "" } : {}),
    },
    [
      stage,
      ...(svg.caption
        ? [element("figcaption", { className: ["media-caption"] }, [inlineText(svg.caption)])]
        : []),
    ],
  );
}

function tweetDate(value: TweetData["createdAt"]): string {
  const date = new Date(
    value.kind === "unix-seconds"
      ? value.value * 1000
      : value.kind === "unix-milliseconds"
        ? value.value
        : value.value,
  );
  if (Number.isNaN(date.getTime())) {
    if (value.kind === "original") return value.value;
    throw new Error("Invalid tweet timestamp");
  }
  const jst = new Date(date.getTime() + 9 * 60 * 60 * 1000);
  const pad = (n: number): string => String(n).padStart(2, "0");
  return `${jst.getUTCFullYear()}/${pad(jst.getUTCMonth() + 1)}/${pad(jst.getUTCDate())} ${pad(jst.getUTCHours())}:${pad(jst.getUTCMinutes())}`;
}

function tweetCard(href: string, tweet: TweetData, diagnostics: Diagnostic[]): Element {
  // TODO(engine-tweet-fields): エンジンの型には認証・返信先・引用投稿がなく、旧表示の該当部品は復元できない。
  const media = tweet.media.flatMap((item) => {
    const preview = item.kind === "video" ? item.poster : item.delivery;
    if (!preview) {
      diagnostics.push({ level: "warning", message: `Tweet video has no poster: ${href}` });
      return [];
    }
    return [
      {
        type: item.kind === "video" ? ("video" as const) : ("photo" as const),
        url: resolveEngineMedia(preview),
        altText: item.alt,
        width: item.dimensions?.width ?? 0,
        height: item.dimensions?.height ?? 0,
      },
    ];
  });
  const card = buildTweetCard({
    id: tweet.id,
    text: tweet.text,
    authorName: tweet.author.name,
    authorHandle: tweet.author.handle,
    authorAvatarUrl: tweet.author.avatar ? resolveEngineMedia(tweet.author.avatar) : "",
    isVerified: false,
    createdAt: tweetDate(tweet.createdAt),
    favoriteCount: tweet.counts.likes ?? 0,
    conversationCount: tweet.counts.replies ?? 0,
    inReplyToHandle: null,
    media,
    quotedTweet: null,
    permalink: href,
  });
  const prune = (node: Element): void => {
    node.children = node.children.filter((child) => {
      if (child.type !== "element") return true;
      const classes = child.properties.className;
      if (Array.isArray(classes)) {
        if (classes.includes("tweet-card__avatar") && !tweet.author.avatar) return false;
        if (classes.includes("tweet-card__stat--like") && tweet.counts.likes === null) return false;
        if (classes.includes("tweet-card__stat--reply") && tweet.counts.replies === null)
          return false;
      }
      if (child.tagName === "img") {
        if (child.properties.width === "0") delete child.properties.width;
        if (child.properties.height === "0") delete child.properties.height;
      }
      prune(child);
      return true;
    });
  };
  prune(card);
  return card;
}

export interface RenderArticleBodyResult {
  html: string;
  title: string;
  headings: Heading[];
  /** 項目の意味と必須条件は利用側が決める。 */
  frontmatter: unknown;
  diagnostics: Diagnostic[];
}

interface ArticleTree {
  root: Root;
  title: string;
  headings: Heading[];
  diagnostics: Diagnostic[];
}

/**
 * 操作子・装飾アイコン・本文を隠す指定を出さない。
 * 書き手の意図と取得の状態は属性へ残し、見せ方と操作は利用側のCSSとJSへ委ねる。
 */
function buildArticleTree(segments: RenderSegment[]): ArticleTree {
  const headingId = createHeadingIdGenerator();
  const headings: Heading[] = [];
  const diagnostics: Diagnostic[] = [];
  const seenSections = new Set<string>();
  let title = "";
  let seenTitle = false;
  let imageCount = 0;

  const blocks = (nodes: RenderBlockNode[], tight = false): ElementContent[] =>
    nodes.flatMap((node) => {
      switch (node.kind) {
        case "title": {
          if (seenTitle) throw new Error("A render document must not have multiple titles");
          seenTitle = true;
          title = inlineNodes(node.value.content).map(extractHeadingText).join("");
          return [];
        }
        case "section-heading": {
          const { id, level, fold, content } = node.value;
          if (seenSections.has(id)) throw new Error(`Repeated section reference: ${id}`);
          seenSections.add(id);
          if (level === "h1") throw new Error("A section heading cannot be h1");
          const first = content[0];
          if (
            level === "h2" &&
            (fold !== "none" || (first?.kind === "text" && /^[>v] +(?=\S)/.test(first.value)))
          ) {
            throw new Error(
              "SectionCollapseMarker: only h3 subsections may have a collapse marker.",
            );
          }
          const depth = level === "h2" ? 2 : 3;
          const slug = headingId(depth);
          const heading = element(
            level,
            {
              ...(slug ? { id: slug } : {}),
              ...(depth === 3 && fold !== "none" ? { dataCollapse: fold } : {}),
            },
            inlineNodes(content),
          );
          if (slug) headings.push({ depth, slug, text: extractHeadingText(heading) });
          return [heading];
        }
        case "paragraph":
          return tight
            ? inlineNodes(node.value.content)
            : [element("p", {}, inlineNodes(node.value.content))];
        case "list":
          return [
            element(
              node.value.kind === "ordered" ? "ol" : "ul",
              {},
              node.value.items.map((item) =>
                element("li", {}, blocks(item.blocks, !node.value.spread)),
              ),
            ),
          ];
        case "quote":
          return [element("blockquote", {}, blocks(node.value.blocks))];
        case "thematic-break":
          return [element("hr")];
        case "callout":
          return [
            buildCallout(
              {
                type: node.value.kind,
                labelText: node.value.label ? inlineText(node.value.label).value : node.value.label,
              },
              blocks(node.value.blocks),
            ),
          ];
        case "code":
          return [codeBlock(node.value)];
        case "code-group":
          return [
            element(
              "div",
              { className: ["code-group"] },
              node.value.entries.map((entry) =>
                codeBlock(entry.code, inlineText(entry.label).value),
              ),
            ),
          ];
        case "table": {
          const table = node.value;
          const row = (cells: typeof table.header.cells, tag: string): Element =>
            element(
              "tr",
              {},
              cells.map((cell, index) =>
                element(
                  tag,
                  table.align[index] && table.align[index] !== "unspecified"
                    ? { dataAlign: table.align[index] }
                    : {},
                  inlineNodes(cell.content),
                ),
              ),
            );
          return [
            element("table", {}, [
              element("thead", {}, [row(table.header.cells, "th")]),
              element(
                "tbody",
                {},
                table.rows.map((item) => row(item.cells, "td")),
              ),
            ]),
          ];
        }
        case "svg":
          return [svgFigure(node.value)];
        case "image": {
          const image = node.value;
          const figure = buildImage(
            {
              src: resolveEngineMedia(image.delivery),
              alt: image.alt,
              ratio: image.ratio ? `${image.ratio.width}/${image.ratio.height}` : null,
              caption: image.caption ? inlineText(image.caption).value : image.caption,
              zoom: image.zoom,
              loop: false,
            },
            { eager: imageCount++ < EAGER_IMAGE_COUNT },
          );
          return [figure];
        }
        case "video": {
          const video = node.value;
          return [
            buildVideo({
              src: resolveEngineMedia(video.delivery),
              alt: null,
              ratio: video.ratio ? `${video.ratio.width}/${video.ratio.height}` : null,
              caption: video.caption ? inlineText(video.caption).value : video.caption,
              zoom: false,
              loop: video.loopPlayback,
            }),
          ];
        }
        case "url-card": {
          const card = buildUrlCard(node.value.href, {
            title: node.value.title.state === "available" ? node.value.title.value : null,
            favicon:
              node.value.icon.state === "available"
                ? resolveEngineMedia(node.value.icon.value)
                : null,
          });
          card.properties.dataIconState = node.value.icon.state;
          return [card];
        }
        case "tweet-card": {
          if (node.value.state.state === "available") {
            return [tweetCard(node.value.href, node.value.state.value, diagnostics)];
          }
          const card = buildUrlCard(node.value.href, { title: null, favicon: null });
          card.properties.dataIconState = "missing";
          return [card];
        }
        default:
          return unreachable(node);
      }
    });

  const root: Root = {
    type: "root",
    children: segments.flatMap((segment): ElementContent[] => {
      switch (segment.content.kind) {
        case "frontmatter":
          return [];
        case "block":
          return blocks([segment.content.value]);
        case "rejected":
          // 規約に反したセグメントは部品にせず、原文をそのまま見せる。改行の保ち方はテーマが決める。
          return [element("p", { dataRejected: "" }, [text(segment.content.value.source)])];
        default:
          return unreachable(segment.content);
      }
    }),
  };
  rehypeTableWrap()(root);
  rehypeHeadingPhraseWrap()(root);
  rehypeArticleStructure()(root);
  return { root, title, headings, diagnostics };
}

/** 本文のHTMLとフックだけを返す。CSS・JS・テーマを受け取らず、ページの枠も作らない。 */
export function renderArticleBody(segments: RenderSegment[]): RenderArticleBodyResult {
  const { root, title, headings, diagnostics } = buildArticleTree(segments);
  return {
    html: toHtml(root, { allowDangerousHtml: true }),
    title,
    headings,
    frontmatter: readEngineFrontmatter(segments),
    diagnostics,
  };
}
