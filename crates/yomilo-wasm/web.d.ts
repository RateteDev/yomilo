import type * as api from "./index.js";

export type * from "./index.js";
export { YomiloError } from "./index.js";

export type Yomilo = Omit<typeof api, "YomiloError">;

/**
 * 渡したWASMの本体でYomiloを初期化し、Node向けの入口と同じ関数を返す。
 * 本体は`@yomilo/wasm/yomilo_wasm_bg.wasm`で、Workersならimportしたモジュール、ブラウザなら取得したバイト列を渡す。
 * 初期化は最初の1回だけ行われる。
 */
export function createYomilo(module: WebAssembly.Module | BufferSource): Yomilo;
