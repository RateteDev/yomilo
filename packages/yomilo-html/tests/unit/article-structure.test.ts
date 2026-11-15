import { expect, test } from "bun:test";
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs } from "@yomilo/wasm";
import type { Element, Root, RootContent } from "hast";
import { fromHtmlIsomorphic } from "hast-util-from-html-isomorphic";
import { prepareSource } from "../utils/prepare.js";

function render(source: string) {
  const inputs = defaultPreparationInputs();
  for (const name of ["a.png", "b.png"]) {
    inputs.images[name] = { kind: "reference", value: { reference: name } };
  }
  inputs.videos["a.mp4"] = { kind: "reference", value: { reference: "a.mp4" } };
  return renderArticleBody(prepareSource(source, inputs));
}

function elements(node: Root | Element): Element[] {
  return node.children.flatMap((child: RootContent) =>
    child.type === "element" ? [child, ...elements(child)] : [],
  );
}

function byId(tree: Root, id: string): Element {
  const element = elements(tree).find((element) => element.properties.id === id);
  if (!element) throw new Error(`Missing element ${id}`);
  return element;
}

function textOf(node: Root | RootContent): string {
  if (node.type === "text") return node.value;
  return "children" in node ? node.children.map(textOf).join("") : "";
}

test("keeps public heading IDs positional and attaches them to the section containing the body", async () => {
  const result = render(
    "# Title\n\nIntroduction\n\n## Repeated section\n\nFirst body\n\n### Repeated subsection\n\nFirst detail\n\n## Repeated section\n\nSecond body\n\n### Repeated subsection\n\nSecond detail",
  );
  expect(result.headings).toEqual([
    { depth: 2, text: "Repeated section", slug: "section-01" },
    { depth: 3, text: "Repeated subsection", slug: "section-01-01" },
    { depth: 2, text: "Repeated section", slug: "section-02" },
    { depth: 3, text: "Repeated subsection", slug: "section-02-01" },
  ]);
  const tree = fromHtmlIsomorphic(result.html, { fragment: true });
  const first = byId(tree, "section-01");
  expect(first.tagName).toBe("section");
  expect(first.children).toContain(byId(tree, "section-01-01"));
  expect(textOf(first)).toContain("First detail");
  expect(textOf(first)).not.toContain("Second body");
  expect(textOf(byId(tree, "section-02"))).toContain("Second detail");
  const headings = elements(tree).filter((element) => /^h[123]$/.test(element.tagName));
  expect(headings.every((heading) => heading.properties.id === undefined)).toBe(true);
});

test("numbers each element kind in document order, including callout contents and code groups", async () => {
  const result = render(
    [
      ':::callout{type="note"}',
      "",
      "```text",
      "Nested code",
      "```",
      "",
      ":::",
      "",
      '::image{src="a.png"}',
      "",
      '::image{src="b.png"}',
      "",
      '::video{src="a.mp4"}',
      "",
      ':::svg{alt="図"}',
      '<svg viewBox="0 0 10 10"><rect width="1" height="1" /></svg>',
      ":::",
      "",
      "```ts",
      "// 日本語のコメント",
      "```",
      "",
      ":::code-group",
      "",
      "```json 設定ファイル",
      '{ "a": 1 }',
      "```",
      "",
      ":::",
      "",
      "| a |",
      "|---|",
      "| b |",
      "",
    ].join("\n"),
  );
  const tree = fromHtmlIsomorphic(result.html, { fragment: true });
  expect(
    elements(tree)
      .map((element) => element.properties.id)
      .filter(Boolean),
  ).toEqual([
    "callout-001",
    "code-block-001",
    "image-001",
    "image-002",
    "video-001",
    "svg-001",
    "code-block-002",
    "code-group-001",
    "code-block-003",
    "table-001",
  ]);
});

test("keeps public heading numbers within two digits and resets subsection counts", () => {
  const sections = Array.from({ length: 99 }, (_, index) => `## Section ${index + 1}`).join("\n\n");
  expect(render(`${sections}\n`).headings.at(-1)?.slug).toBe("section-99");
  expect(() => render(`${sections}\n\n## Section 100\n\n## Section 101\n`)).toThrow(
    "IdNumberOverflow",
  );
  const subsections = Array.from({ length: 99 }, (_, index) => `### Subsection ${index + 1}`).join(
    "\n\n",
  );
  const source = `## First\n\n${subsections}\n\n## Second\n\n${subsections}\n`;
  expect(render(source).headings.at(-1)?.slug).toBe("section-02-99");
  expect(() => render(`${source}\n### Subsection 100\n`)).toThrow("IdNumberOverflow");
});

test("rejects the first element number beyond three digits", () => {
  const tables = Array.from({ length: 999 }, () => "| a |\n|---|\n| b |").join("\n\n");
  expect(render(`${tables}\n`).html).toContain('id="table-999"');
  expect(() => render(`${tables}\n\n| a |\n|---|\n| b |\n`)).toThrow("IdNumberOverflow");
});
