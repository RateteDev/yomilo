/**
 * The closed vocabulary an article frontmatter may carry, and the parser that
 * reads it. The keys follow the web app's article metadata specification;
 * the values are shared, while how each renderer uses them is not.
 *
 * Every field is optional here. A document is expected to be brought to this
 * specification before it is opened, but requiring a field would refuse a
 * document the renderer can draw; the web app layers its own required set on
 * top through its content schema.
 *
 * The vocabulary is closed for the web app only. A document opened by the CLI
 * may carry keys of its own — a reviewer, an internal ticket — and those are
 * left alone here: they are shown only when the CLI is asked to, and the web
 * app's content schema refuses them before this parser is reached. A
 * specified field carrying a malformed value is an error, because the
 * specification is the one thing the document was supposed to be brought to.
 */
import { parse as parseYaml } from "yaml";

/** Where an article sits on the single path towards publication. */
export const ARTICLE_STATUSES = ["draft", "published", "archived", "deleted"] as const;

export type ArticleStatus = (typeof ARTICLE_STATUSES)[number];

/** Status assumed for a document that carries no `status`. */
export const DEFAULT_ARTICLE_STATUS: ArticleStatus = "published";

export interface Frontmatter {
  /** Sole classification the article belongs to, written as the label itself. */
  category?: string;
  /**
   * Summary of the article. Read by people under the title and on list cards,
   * and handed to meta description, feeds and share cards.
   */
  description?: string;
  author?: string;
  /** Publication instant in ISO 8601, offset included. */
  published?: string;
  /** Last modification instant in ISO 8601, offset included. */
  updated?: string;
  tags: string[];
  /** Site-absolute path to the 16:9 image representing the article. */
  image?: string;
  status: ArticleStatus;
  /** Instant the owner last proof-read the body, in ISO 8601. */
  reviewed?: string;
}

export type ArticleFrontmatterKey = keyof Frontmatter;

/** Single source of the vocabulary. */
export const ARTICLE_FRONTMATTER_KEYS = [
  "category",
  "description",
  "author",
  "published",
  "updated",
  "tags",
  "image",
  "status",
  "reviewed",
] as const satisfies readonly ArticleFrontmatterKey[];

/**
 * What the article header shows whenever the document carries it, on the web
 * and in the CLI alike. Anything else reaches the CLI's header only by name.
 */
export const HEADER_FRONTMATTER_KEYS = [
  "description",
  "tags",
  "published",
] as const satisfies readonly ArticleFrontmatterKey[];

/** Fields holding an instant in ISO 8601, displayed as a date rather than as written. */
export const DATE_FRONTMATTER_KEYS = [
  "published",
  "updated",
  "reviewed",
] as const satisfies readonly ArticleFrontmatterKey[];

/** Fields carrying a plain string, all read the same way. */
const STRING_KEYS = [
  "category",
  "description",
  "author",
  "published",
  "updated",
  "image",
  "reviewed",
] as const satisfies readonly ArticleFrontmatterKey[];

export function parseFrontmatter(source: string): { frontmatter: unknown; body: string } {
  const match = source.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/);
  if (!match?.[1] || match[2] === undefined) {
    return { frontmatter: {}, body: source };
  }
  const frontmatter = parseYaml(match[1]);
  return { frontmatter, body: match[2] };
}

function readString(fm: Record<string, unknown>, key: ArticleFrontmatterKey): string | undefined {
  const value = fm[key];
  if (value === undefined || value === null) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Frontmatter field "${key}" must be a string`);
  }
  return value;
}

function readTags(value: unknown): string[] {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value) || value.some((tag) => typeof tag !== "string")) {
    throw new Error('Frontmatter field "tags" must be an array of strings');
  }
  return value as string[];
}

function readStatus(value: unknown): ArticleStatus {
  if (value === undefined || value === null) {
    return DEFAULT_ARTICLE_STATUS;
  }
  if (typeof value !== "string" || !(ARTICLE_STATUSES as readonly string[]).includes(value)) {
    throw new Error(
      `Frontmatter field "status" must be one of ${ARTICLE_STATUSES.join(", ")}, got ${JSON.stringify(value)}`,
    );
  }
  return value as ArticleStatus;
}

/**
 * Reads a parsed frontmatter into the shared shape. Keys outside the
 * specification are not part of that shape and are passed over; a malformed
 * value on a specified field throws. Null when the document carries no
 * frontmatter, which is the one case with no header to render.
 */
export function validateFrontmatter(raw: unknown): Frontmatter | null {
  if (typeof raw !== "object" || raw === null || Object.keys(raw).length === 0) {
    return null;
  }
  const fm = raw as Record<string, unknown>;

  const frontmatter: Frontmatter = {
    tags: readTags(fm.tags),
    status: readStatus(fm.status),
  };
  for (const key of STRING_KEYS) {
    const value = readString(fm, key);
    if (value !== undefined) {
      frontmatter[key] = value;
    }
  }
  return frontmatter;
}
