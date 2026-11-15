import { expect, test } from "bun:test";
import type { ElementContent } from "hast";
import { toHtml } from "hast-util-to-html";
import { normalizeEngineInlineSoftbreaks } from "../../src/engine-inline-softbreaks.js";

test("inline code whitespace is immutable even when it participates in a visible-character boundary", () => {
  const nodes: ElementContent[] = [
    { type: "text", value: "前。\n" },
    {
      type: "element",
      tagName: "code",
      properties: {},
      children: [{ type: "text", value: " 日本\n語 " }],
    },
    { type: "text", value: "\n後。 語間 空白" },
  ];
  normalizeEngineInlineSoftbreaks(nodes);
  expect(toHtml({ type: "root", children: nodes })).toBe(
    "前。<code> 日本\n語 </code>後。 語間 空白",
  );

  const codeOnlyBreak: ElementContent[] = [
    { type: "text", value: "前 " },
    {
      type: "element",
      tagName: "code",
      properties: {},
      children: [{ type: "text", value: "\n" }],
    },
    { type: "text", value: " 後" },
  ];
  normalizeEngineInlineSoftbreaks(codeOnlyBreak);
  expect(toHtml({ type: "root", children: codeOnlyBreak })).toBe("前 <code>\n</code> 後");
});
