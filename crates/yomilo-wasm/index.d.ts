import type { CanonicalCheckResult } from "./types/CanonicalCheckResult.js";
import type { Cursor } from "./types/Cursor.js";
import type { InputState } from "./types/InputState.js";
import type { JsonApiError } from "./types/JsonApiError.js";
import type { PreparationInputs } from "./types/PreparationInputs.js";
import type { RenderSegment } from "./types/RenderSegment.js";
import type { RequiredInputs } from "./types/RequiredInputs.js";
import type { Segment } from "./types/Segment.js";
import type { SegmentBatch } from "./types/SegmentBatch.js";
import type { TweetState } from "./types/TweetState.js";

export type { Diagnostic } from "./types/Diagnostic.js";
export type { MediaDelivery } from "./types/MediaDelivery.js";
export type { RenderBlockNode } from "./types/RenderBlockNode.js";
export type { TweetData } from "./types/TweetData.js";
export type {
  CanonicalCheckResult,
  Cursor,
  InputState,
  JsonApiError,
  PreparationInputs,
  RenderSegment,
  RequiredInputs,
  Segment,
  SegmentBatch,
  TweetState,
};

export class YomiloError extends Error {
  readonly details: JsonApiError;
  constructor(details: JsonApiError);
}

/**
 * 原稿を文書の直下の要素ごとのセグメントに分け、今回新しく確定したものを返す。
 * 毎回原稿の全文を渡し、前回の`cursor`を添えると未確定の部分だけを解析し直す。最初の呼び出しは`null`を渡す。
 * `"partial"`では最後の要素を未確定として`pending`に残し、`"complete"`ではすべてを確定する。
 */
export function segment(
  source: string,
  cursor: Cursor | null,
  input: InputState,
): SegmentBatch;
/** セグメントが描画準備に求める画像・動画・カードを返す。取得も読み込みもしない。 */
export function requiredInputs(segment: Segment): RequiredInputs;
/** 確定したセグメントを1つ描画準備する。前後のセグメントには依存しない。 */
export function prepareSegment(
  segment: Segment,
  inputs: PreparationInputs,
): RenderSegment;
/** 取得したページのHTMLからURLカードの題名を読む。無ければnull。 */
export function interpretUrlCardPage(html: string): string | null;
/** Xの`tweet-result`の応答を読む。画像は参照で返す。 */
export function interpretTweetResult(id: string, json: string): TweetState;
export function formatSource(source: string): string;
export function checkCanonical(source: string): CanonicalCheckResult;
export function defaultPreparationInputs(): PreparationInputs;
