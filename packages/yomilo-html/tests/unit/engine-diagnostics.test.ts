import { expect, test } from "bun:test";
import { formatEngineError, formatRejectedSegments, type JsonApiError } from "@yomilo/html/engine";
import { defaultPreparationInputs, prepareSegment, segment, YomiloError } from "@yomilo/wasm";

function preparationFailure(source: string, inputs = defaultPreparationInputs()): JsonApiError {
  try {
    for (const item of segment(source, null, "complete").segments) prepareSegment(item, inputs);
  } catch (error) {
    if (error instanceof YomiloError) return error.details;
    throw error;
  }
  throw new Error("Expected engine failure");
}

test("formats every rejected segment's diagnostics with source positions and related declarations", () => {
  const rejected = segment("# Title\n\n#### Too deep\n\n![図](image.png)\n", null, "complete");
  const message = formatRejectedSegments("article.md", rejected.segments);
  expect(message).toStartWith("article.md\n  structure: 2 segment(s)");
  expect(message).toContain(
    "3:1  unsupported unsupported-syntax: Only h1, h2, and h3 headings are supported",
  );
  expect(message).toContain(
    "5:1  unsupported unsupported-syntax: Markdown images are not supported",
  );
  const duplicate = segment('# Title\n\n::image{src="a.png" src="b.png"}\n', null, "complete");
  const related = formatRejectedSegments("duplicate.md", duplicate.segments);
  expect(related).toContain("related: First declaration");
  expect(related).toContain("3:9");
  expect(formatRejectedSegments("valid.md", segment("# Title\n", null, "complete").segments)).toBe(
    null,
  );
});

test("formats preparation locations and does not invent a source location when none exists", () => {
  expect(
    formatEngineError("article.md", preparationFailure('# Title\n\n::image{src="missing.png"}\n')),
  ).toContain('3:1  semantic missing-media-input: Missing image delivery input for "missing.png"');
  const inputs = defaultPreparationInputs();
  inputs.versions.syntaxVersion += 1;
  const error = preparationFailure("# Title\n", inputs);
  expect(error.preparation?.kind).toBe("version-mismatch");
  expect(formatEngineError("article.md", error)).toContain("?:?");
});
