import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { createYomilo } from "./api.js";

export { YomiloError } from "./api.js";

// Astroのバンドル後もnpmパッケージを基準にWASMを解決する。
const wasm = readFileSync(createRequire(import.meta.url).resolve("@yomilo/wasm/yomilo_wasm_bg.wasm"));

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
