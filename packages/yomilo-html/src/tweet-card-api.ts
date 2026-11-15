import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { interpretTweetResult, type TweetState } from "@yomilo/wasm";
import { cacheDir, type FetchCacheOptions } from "./cache-dir";
import { FETCH_TIMEOUT_MS } from "./fetch-constants";

const SYNDICATION_URL = "https://cdn.syndication.twimg.com/tweet-result";
const FEATURES = [
  "tfw_timeline_list:",
  "tfw_follower_count_sunset:true",
  "tfw_tweet_edit_backend:on",
  "tfw_refsrc_session:on",
  "tfw_fosnr_soft_interventions_enabled:on",
  "tfw_show_birdwatch_pivots_enabled:on",
  "tfw_show_business_verified_badge:on",
  "tfw_duplicate_scribes_to_settings:on",
  "tfw_use_profile_image_shape_enabled:on",
  "tfw_show_blue_verified_badge:on",
  "tfw_legacy_timeline_sunset:true",
  "tfw_show_gov_verified_badge:on",
  "tfw_show_business_affiliate_badge:on",
  "tfw_tweet_edit_frontend:on",
].join(";");

// 応答をそのまま保存し、Coreの読み方が変わっても取得し直さずに読み直せるようにする。
const CACHE_KIND = "tweet-result";

function cachePath(options: FetchCacheOptions, id: string): string {
  return join(
    cacheDir(options, CACHE_KIND),
    `${createHash("sha256").update(id).digest("hex").slice(0, 16)}.json`,
  );
}

function getToken(id: string): string {
  return ((Number(id) / 1e15) * Math.PI).toString(36).replace(/(0+|\.)/g, "");
}

async function fetchTweetResult(
  tweetId: string,
  options: FetchCacheOptions,
): Promise<string | TweetState> {
  const mode = options.cache ?? "off";
  const file = cachePath(options, tweetId);
  if (mode === "use" && existsSync(file)) return readFileSync(file, "utf-8");
  const url = new URL(SYNDICATION_URL);
  url.searchParams.set("id", tweetId);
  url.searchParams.set("lang", "ja");
  url.searchParams.set("features", FEATURES);
  url.searchParams.set("token", getToken(tweetId));
  const res = await fetch(url.href, {
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    headers: { "User-Agent": "bot" },
  });
  if (res.status === 404) return { state: "unavailable", value: { reason: { kind: "missing" } } };
  if (!res.ok) {
    return {
      state: "unavailable",
      value: { reason: { kind: "failed", value: { message: `HTTP ${res.status}` } } },
    };
  }
  const text = await res.text();
  if (mode !== "off") {
    mkdirSync(cacheDir(options, CACHE_KIND), { recursive: true });
    writeFileSync(file, text);
  }
  return text;
}

/** 画像は参照で返る。埋め込む利用側は、取得した中身へ差し替える。 */
export async function fetchTweet(
  tweetId: string,
  options: FetchCacheOptions = {},
): Promise<TweetState> {
  try {
    const result = await fetchTweetResult(tweetId, options);
    return typeof result === "string" ? interpretTweetResult(tweetId, result) : result;
  } catch (error) {
    return {
      state: "unavailable",
      value: { reason: { kind: "failed", value: { message: String(error) } } },
    };
  }
}
