import * as raw from "./pkg/yomilo_wasm.js";

export class YomiloError extends Error {
  constructor(details) {
    super(details.message);
    this.name = "YomiloError";
    this.details = details;
  }
}

function invoke(call) {
  try {
    return call();
  } catch (error) {
    if (!(error instanceof Error)) throw error;
    let details;
    try {
      details = JSON.parse(error.message);
    } catch {
      throw error;
    }
    throw new YomiloError(details);
  }
}

// WASMの読み込み方は実行環境ごとに違う（Nodeはファイル、Workersはimportしたモジュール、ブラウザは取得したバイト列）ため、
// 本体の受け取りを利用側に任せ、呼び方だけをここで揃える。
export function createYomilo(module) {
  raw.initSync({ module });
  return {
    segment: (source, cursor, input) =>
      invoke(() => JSON.parse(raw.segment(source, JSON.stringify(cursor), input))),
    requiredInputs: (segment) =>
      invoke(() => JSON.parse(raw.requiredInputs(JSON.stringify(segment)))),
    prepareSegment: (segment, inputs) =>
      invoke(() =>
        JSON.parse(raw.prepareSegment(JSON.stringify(segment), JSON.stringify(inputs))),
      ),
    interpretUrlCardPage: (html) => invoke(() => JSON.parse(raw.interpretUrlCardPage(html))),
    interpretTweetResult: (id, json) =>
      invoke(() => JSON.parse(raw.interpretTweetResult(id, json))),
    formatSource: (source) => invoke(() => raw.formatSource(source)),
    checkCanonical: (source) => invoke(() => JSON.parse(raw.checkCanonical(source))),
    defaultPreparationInputs: () => invoke(() => JSON.parse(raw.defaultPreparationInputs())),
  };
}
