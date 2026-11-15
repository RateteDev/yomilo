import { type PreparationInputs, prepareSegment, type RenderSegment, segment } from "@yomilo/wasm";

/** 原稿の全文を一度に確定させ、各セグメントを描画準備する。 */
export function prepareSource(source: string, inputs: PreparationInputs): RenderSegment[] {
  return segment(source, null, "complete").segments.map((item) => prepareSegment(item, inputs));
}
