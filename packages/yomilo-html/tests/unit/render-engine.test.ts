import { expect, test } from "bun:test";
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs } from "@yomilo/wasm";
import { fromHtmlIsomorphic } from "hast-util-from-html-isomorphic";
import { prepareSource } from "../utils/prepare.js";

const SOURCE = `---
description: 要約
published: "2026-08-06T21:00:00+09:00"
tags: [sample]
reviewer: "<Reviewer>"
ticket: INTERNAL-42
---

# Title

## Section

本文と\`mono\`。

### > Details

詳細。
`;

test("rendering preserves Unicode and authored whitespace in highlighted, plain and grouped code", () => {
  const code = '// 日本語🙂\n\tconst message = "こんにちは🙂";  \n\nconsole.log(message);';
  const plain = "\tプレーン🙂  \n\n  末尾\t";
  const source = `# Code\n\n\`\`\`typescript\n${code}\n\`\`\`\n\n:::code-group\n\n\`\`\`typescript TS\n${code}\n\`\`\`\n\n\`\`\`text Text\n${plain}\n\`\`\`\n\n:::\n`;
  const document = prepareSource(source, defaultPreparationInputs());
  const article = renderArticleBody(document);
  const root = fromHtmlIsomorphic(article.html, { fragment: true });
  expect(article.html).toContain('data-code-role="comment"');
  expect(article.html).toContain('data-code-role="keyword"');
  expect(article.html).not.toMatch(/<(?:pre|span)\b[^>]*\bstyle=/);
  const blocks: string[] = [];
  const visit = (node: typeof root | (typeof root.children)[number]): void => {
    if (node.type === "element" && node.tagName === "pre") {
      const content = (child: (typeof root.children)[number]): string => {
        if (child.type === "text") return child.value;
        return "children" in child ? child.children.map(content).join("") : "";
      };
      blocks.push(content(node));
    } else if ("children" in node) {
      for (const child of node.children) visit(child);
    }
  };
  visit(root);
  expect(blocks).toEqual([code, code, plain]);
});

test("the body-only API shows all authored content without controls, icons or hidden parts", () => {
  const inputs = defaultPreparationInputs();
  inputs.urlCards["https://example.com/"] = {
    title: { state: "failed", value: { message: "offline" } },
    icon: { state: "failed", value: { message: "offline" } },
  };
  const source = `${SOURCE}
:::callout{type="caution" label="caution"}
危険。
:::

:::code-group

\`\`\`typescript TS
const a = 1;
\`\`\`

\`\`\`text Plain
${"line\n".repeat(20)}\`\`\`

:::

::url-card{href="https://example.com/"}

:::svg{alt="Diagram" zoom}
<svg viewBox="0 0 10 10"><rect width="8" height="6" fill="currentColor" /></svg>
:::
`;
  const result = renderArticleBody(prepareSource(source, inputs));
  expect(result.title).toBe("Title");
  expect(result.html).not.toMatch(/<(?:button|script|style|link)\b|\shidden\b|\sstyle=/);
  expect(result.html.match(/<svg\b/g)).toHaveLength(1);
  expect(result.html).toContain('<section id="section-01-01" data-fold="closed"><h3>Details</h3>');
  expect(result.html).toContain("詳細。");
  expect(result.html).toContain('<span class="code-group-label">TS</span>');
  expect(result.html).toContain('<span class="code-group-label">Plain</span>');
  expect(result.html).toContain('data-icon-state="failed"');
  expect(result.html).toContain('data-zoom=""');
  expect(result.html).toMatch(/class="callout caution"[^>]*><span class="callout-label">caution</);
});

test("a rejected segment is shown as its raw source while the surrounding segments still render", () => {
  const source = "## 節\n\nこれは**重要。**次の<b>文</b>。\n\n- 後の項目\n";
  const html = renderArticleBody(prepareSource(source, defaultPreparationInputs())).html;
  expect(html).toContain("<h2>節</h2>");
  expect(html).toContain('<p data-rejected="">これは**重要。**次の&#x3C;b>文&#x3C;/b>。</p>');
  expect(html).toContain("<li>後の項目</li>");
});
