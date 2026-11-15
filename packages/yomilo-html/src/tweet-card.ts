import type { TweetState } from "@yomilo/wasm";

const TWEET_URL_RE = /^https?:\/\/(?:www\.)?(?:twitter\.com|x\.com)\/\w+\/status\/(\d+)/;

export function extractTweetId(url: string): string | null {
  const match = url.match(TWEET_URL_RE);
  return match ? (match[1] as string) : null;
}

export { buildTweetCard } from "./tweet-card-build.js";
export type { QuotedTweetData, TweetData, TweetMedia } from "./tweet-card-data.js";

export type FetchTweet = (tweetId: string) => Promise<TweetState>;
