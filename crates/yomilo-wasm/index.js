import { createRequire } from "node:module";

// Astroのバンドル後もnpmパッケージを基準にWASMを解決する。
const raw = createRequire(import.meta.url)("@yomilo/wasm/raw");

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

export function segment(source, cursor, input) {
  return invoke(() =>
    JSON.parse(raw.segment(source, JSON.stringify(cursor), input)),
  );
}

export function requiredInputs(segment) {
  return invoke(() => JSON.parse(raw.requiredInputs(JSON.stringify(segment))));
}

export function prepareSegment(segment, inputs) {
  return invoke(() =>
    JSON.parse(
      raw.prepareSegment(JSON.stringify(segment), JSON.stringify(inputs)),
    ),
  );
}

export function interpretUrlCardPage(html) {
  return invoke(() => JSON.parse(raw.interpretUrlCardPage(html)));
}

export function interpretTweetResult(id, json) {
  return invoke(() => JSON.parse(raw.interpretTweetResult(id, json)));
}

export function formatSource(source) {
  return invoke(() => raw.formatSource(source));
}

export function checkCanonical(source) {
  return invoke(() => JSON.parse(raw.checkCanonical(source)));
}

export function defaultPreparationInputs() {
  return invoke(() => JSON.parse(raw.defaultPreparationInputs()));
}
