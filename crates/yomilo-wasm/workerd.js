// Workersでは既定の入口が使うNodeのファイル読み込みが無いため、wranglerが選ぶworkerd条件でこの入口を使う。
// wranglerはWASMのimportを読み込み済みのモジュールとして渡す。
import wasm from "./pkg/yomilo_wasm_bg.wasm";
import { createYomilo } from "./api.js";

export { YomiloError } from "./api.js";

export const {
  segment,
  requiredInputs,
  prepareSegment,
  interpretUrlCardPage,
  interpretTweetResult,
  formatSource,
  checkCanonical,
  defaultPreparationInputs,
} = createYomilo(wasm);
