/**
 * Fetch OGP metadata for a URL, optionally kept in a file cache (see
 * `FetchCacheOptions`).
 *
 * The favicon comes from Google's favicon service rather than the page's own
 * `<link rel="icon">`: sites declare icons in too many places (link variants,
 * a bare /favicon.ico, the manifest) to chase each one. It is fetched at build
 * time, and both its address and its bytes are kept: a caller that embeds the
 * bytes spares the reader's browser from contacting Google or the linked site,
 * and a caller that references the address keeps its output small.
 *
 * The title is read from the page by Core, so every environment reads it alike.
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { interpretUrlCardPage } from "@yomilo/wasm";
import { cacheDir, type FetchCacheOptions } from "./cache-dir";
import { FETCH_TIMEOUT_MS } from "./fetch-constants";

/** Media read at fetch time. `bytes` is base64. */
export interface FetchedMedia {
  url: string;
  mediaType: string;
  bytes: string;
}

export interface OgpData {
  title: string | null;
  favicon: FetchedMedia | null;
}

const CACHE_KIND = "url-card-ogp";

function cacheKey(url: string): string {
  return createHash("sha256").update(url).digest("hex").slice(0, 16);
}

function readCache(directory: string, url: string): OgpData | null {
  const file = join(directory, `${cacheKey(url)}.json`);
  if (!existsSync(file)) return null;
  try {
    const raw = JSON.parse(readFileSync(file, "utf-8"));
    // Entries written before the address was kept hold a data URI. Fetch those again.
    if (typeof raw.favicon === "string") return null;
    return { title: raw.title, favicon: raw.favicon };
  } catch {
    return null;
  }
}

function writeCache(directory: string, url: string, data: OgpData): void {
  mkdirSync(directory, { recursive: true });
  const file = join(directory, `${cacheKey(url)}.json`);
  writeFileSync(file, JSON.stringify({ ...data, _url: url, _ts: Date.now() }));
}

const FAVICON_SIZE = 64;

async function fetchPageMeta(url: string): Promise<Omit<OgpData, "favicon">> {
  try {
    const res = await fetch(url, {
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
      headers: { "User-Agent": "bot" },
    });
    if (!res.ok) return { title: null };
    return { title: interpretUrlCardPage(await res.text()) };
  } catch {
    return { title: null };
  }
}

/**
 * Google answers a domain it has no icon for with 404 and a generic globe
 * image. Treating that as "no favicon" keeps the globe off the card.
 */
async function fetchFavicon(url: string): Promise<FetchedMedia | null> {
  try {
    const { hostname } = new URL(url);
    const faviconUrl = `https://www.google.com/s2/favicons?sz=${FAVICON_SIZE}&domain=${encodeURIComponent(hostname)}`;
    const res = await fetch(faviconUrl, { signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
    if (!res.ok) return null;
    const mediaType = res.headers.get("content-type")?.split(";")[0] ?? "image/png";
    const body = Buffer.from(await res.arrayBuffer());
    return { url: faviconUrl, mediaType, bytes: body.toString("base64") };
  } catch {
    return null;
  }
}

export async function fetchOgp(url: string, options: FetchCacheOptions = {}): Promise<OgpData> {
  const mode = options.cache ?? "off";
  const directory = cacheDir(options, CACHE_KIND);
  const cached = mode === "use" ? readCache(directory, url) : null;
  if (cached) return cached;

  const [meta, favicon] = await Promise.all([fetchPageMeta(url), fetchFavicon(url)]);
  const data: OgpData = { ...meta, favicon };
  if (mode !== "off") writeCache(directory, url, data);
  return data;
}
