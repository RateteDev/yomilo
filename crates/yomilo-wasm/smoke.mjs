import { strict as assert } from "node:assert";
import { readFile } from "node:fs/promises";
import {
  checkCanonical,
  defaultPreparationInputs,
  formatSource,
  prepareSegment,
  requiredInputs,
  segment,
  YomiloError,
} from "./index.js";
// index.jsが初期化した同じ読み込み口を、包む前の出力との比較に使う。
import * as raw from "./pkg/yomilo_wasm.js";

const path = process.argv[2];
if (!path) throw new Error("Usage: node smoke.mjs <markdown-file>");
const source = await readFile(path, "utf8");

const batch = segment(source, null, "complete");
assert.deepEqual(batch, JSON.parse(raw.segment(source, "null", "complete")));
assert.equal(batch.pending, "");
for (const item of batch.segments) assert.notEqual(item.content.kind, "rejected");

// エンジンが挙げた入力だけを、スモーク用の配送結果で補う。
const inputs = defaultPreparationInputs();
for (const item of batch.segments) {
  const required = requiredInputs(item);
  for (const reference of required.images)
    inputs.images[reference] = { kind: "reference", value: { reference } };
  for (const reference of required.videos)
    inputs.videos[reference] = {
      kind: "embedded",
      value: { mediaType: "video/mp4", bytes: "AAAAAA==" },
    };
  for (const href of required.urlCards)
    inputs.urlCards[href] = {
      title: { state: "available", value: "Smoke title" },
      icon: { state: "missing" },
    };
  for (const href of required.tweetCards)
    inputs.tweetCards[href] = {
      state: "unavailable",
      value: { reason: { kind: "missing" } },
    };
}

const ready = batch.segments.map((item) => prepareSegment(item, inputs));
const json = JSON.stringify(
  batch.segments.map((item) =>
    JSON.parse(raw.prepareSegment(JSON.stringify(item), JSON.stringify(inputs))),
  ),
);
assert.deepEqual(ready, JSON.parse(json));
const canonical = formatSource(source);
assert.equal(formatSource(canonical), canonical);
assert.equal(checkCanonical(canonical).canonical, true);

function visit(blocks, callback) {
  for (const block of blocks) {
    callback(block);
    if (block.kind === "list")
      for (const item of block.value.items) visit(item.blocks, callback);
    if (block.kind === "quote" || block.kind === "callout")
      visit(block.value.blocks, callback);
  }
}
const kinds = {};
visit(
  ready.flatMap((item) => (item.content.kind === "block" ? [item.content.value] : [])),
  (block) => {
    kinds[block.kind] = (kinds[block.kind] ?? 0) + 1;
  },
);
function noSpans(value) {
  if (value === null || typeof value !== "object") return;
  assert.equal(Object.hasOwn(value, "span"), false);
  assert.equal(Object.hasOwn(value, "byteOffset"), false);
  for (const child of Object.values(value)) noSpans(child);
}
noSpans(ready);

const unsupported = segment("#### Unsupported heading\n", null, "complete").segments;
assert.equal(unsupported[0].content.kind, "rejected");
assert.equal(unsupported[0].content.value.diagnostics[0].span.start.line, 1);
try {
  segment("", { ...batch.cursor, offset: 1 }, "partial");
  throw new Error("A cursor outside the source was accepted");
} catch (error) {
  assert.ok(error instanceof YomiloError);
  assert.equal(error.details.kind, "invalid-input");
}

console.error(
  JSON.stringify({
    ok: true,
    jsonBytes: Buffer.byteLength(json),
    segments: ready.length,
    versions: ready[0]?.versions,
    kinds,
  }),
);
process.stdout.write(`${json}\n`);
