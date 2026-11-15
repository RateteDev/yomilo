import type { MediaDelivery, PreparationInputs } from "@yomilo/wasm";
import type { FetchedMedia, OgpData } from "./url-card-ogp.js";

export type UrlCardFetch = PreparationInputs["urlCards"][string];
export type TweetState = PreparationInputs["tweetCards"][string];

export type CardFetchResult<T> =
  | { state: "available"; value: T }
  | { state: "missing" }
  | { state: "failed"; message: string };

/** URLの解決・素材の取得・埋め込みの選択はAdapterが済ませる。取得時に中身を読んだ素材は`fetched`で渡り、埋め込むか参照するかはAdapterが選ぶ。 */
export type CardMediaResolver = (source: string, fetched?: FetchedMedia) => MediaDelivery;

export function toEngineUrlCardFetch(
  result: CardFetchResult<OgpData>,
  resolveMedia: CardMediaResolver,
): UrlCardFetch {
  if (result.state === "failed") {
    const failed = { state: "failed" as const, value: { message: result.message } };
    return { title: failed, icon: failed };
  }
  if (result.state === "missing") {
    return { title: { state: "missing" }, icon: { state: "missing" } };
  }
  const ogp = result.value;
  return {
    title: ogp.title === null ? { state: "missing" } : { state: "available", value: ogp.title },
    icon: ogp.favicon
      ? { state: "available", value: resolveMedia(ogp.favicon.url, ogp.favicon) }
      : { state: "missing" },
  };
}
