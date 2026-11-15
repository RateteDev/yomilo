import { expect, test } from "bun:test";
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs } from "@yomilo/wasm";
import type { Element, Root, RootContent } from "hast";
import { fromHtmlIsomorphic } from "hast-util-from-html-isomorphic";
import { visit } from "unist-util-visit";
import { prepareSource } from "../utils/prepare.js";

function textContent(node: Root | RootContent): string {
  if (node.type === "text") return node.value;
  return "children" in node ? node.children.map(textContent).join("") : "";
}

function elements(html: string, tagName: string): Element[] {
  const result: Element[] = [];
  visit(fromHtmlIsomorphic(html, { fragment: true }), "element", (node) => {
    if (node.tagName === tagName) result.push(node);
  });
  return result;
}

test("real engine softbreaks leave no gap beside Japanese text or punctuation but preserve non-Japanese word boundaries and authored spaces", () => {
  const cases = [
    ["見本である。\n題材は文書。", "見本である。題材は文書。"],
    [
      "かな\nカナ\n漢字\n「引用」。\n！？・\n【全角】\n＄記号",
      "かなカナ漢字「引用」。！？・【全角】＄記号",
    ],
    ["𠮷\n野", "𠮷野"],
    ["日本語。 \n \t次。", "日本語。次。"],
    ["日本語\nABC\n日本語\n123\n日本語", "日本語ABC日本語123日本語"],
    ["見本。\nCJK\n「引用」", "見本。CJK「引用」"],
    ["「\nASCII\n」\n！？\n123\n・", "「ASCII」！？123・"],
    ["日本語。\n./path", "日本語。./path"],
    ["English\nwords", "English words"],
    ["日本語\nＬＡＴＩＮ\n日本語\n１２３", "日本語ＬＡＴＩＮ日本語１２３"],
    ["ＬＡＴＩＮ\n１２３", "ＬＡＴＩＮ １２３"],
    ["日本語\n🙂\n次。", "日本語🙂次。"],
    ["日本 語とEnglish words。", "日本 語とEnglish words。"],
    ["日本\u00a0語。", "日本\u00a0語。"],
  ] as const;
  const source = `# 題\n\n${cases.map(([input]) => input).join("\n\n")}\n`;
  const document = prepareSource(source, defaultPreparationInputs());
  const original = JSON.stringify(document);
  expect(original).toContain("\\n");
  const article = renderArticleBody(document);
  expect(elements(article.html, "p").map(textContent)).toEqual(
    cases.map(([, expected]) => expected),
  );
  expect(JSON.stringify(document)).toBe(original);
});

test("softbreak decisions use visible characters across strong, emphasis, code and link boundaries without changing URLs or code", () => {
  const href = "https://example.test/path?q=one%20two&n=1#part";
  const paragraphs = [
    "前。\n**強調**\n後。",
    "前。\n`漢字`\n後。",
    `前。\n[表示](${href})\n後。`,
    "**前。**\n*後。*",
    `**日本\n語**。\n[リンク\n内](${href})。`,
    `前。\n**ABC**\n後。\n\`123\`\n[link](${href})\n次。`,
    "`日本 語`\n日本語。",
    "日本 **語**と[語 間](https://example.test/)の空白。",
    "日本語&#32;&#10;**&#32;次**",
    "English&#32;&#9;&#10;**&#32;words**",
    `**前&#32;&#10;**&#32;[次](${href})`,
    `English\n**words**\n\`123\`\n[link](${href})\nASCII`,
    `**CJK**\n。\n\`ASCII\`\n、\n[link](${href})\n「次」`,
  ];
  const article = renderArticleBody(
    prepareSource(`# 題\n\n${paragraphs.join("\n\n")}\n`, defaultPreparationInputs()),
  );
  expect(elements(article.html, "p").map(textContent)).toEqual([
    "前。強調後。",
    "前。漢字後。",
    "前。表示後。",
    "前。後。",
    "日本語。リンク内。",
    "前。ABC後。123 link次。",
    "日本 語日本語。",
    "日本 語と語 間の空白。",
    "日本語次",
    "English words",
    "前次",
    "English words 123 link ASCII",
    "CJK。ASCII、link「次」",
  ]);
  expect(elements(article.html, "code").map(textContent)).toEqual([
    "漢字",
    "123",
    "日本 語",
    "123",
    "ASCII",
  ]);
  expect(elements(article.html, "a").map((node) => node.properties.href)).toEqual([
    href,
    href,
    href,
    "https://example.test/",
    href,
    href,
    href,
  ]);
});

test("list, quote and callout inline content follows the same softbreak policy without crossing block boundaries", () => {
  const source = `# 題

> 引用。
> **続き。**

- 項目。
  *続き。*
- 次。

:::callout{type="note"}
注意。
[続き。](https://example.test/)

English
words
:::
`;
  const article = renderArticleBody(prepareSource(source, defaultPreparationInputs()));
  expect(elements(article.html, "blockquote").map(textContent)).toEqual(["引用。続き。"]);
  expect(elements(article.html, "li").map(textContent)).toEqual(["項目。続き。", "次。"]);
  expect(elements(article.html, "p").map(textContent)).toEqual([
    "引用。続き。",
    "注意。続き。",
    "English words",
  ]);
});

test("titles, headings, TOC and each table cell normalize their own inline text without merging cells", () => {
  const source = `# 題&#10;名

## 節&#10;次

### 英&#10;ASCII

## English&#10;words

| 日本&#10;語 | English&#10;word |
| --- | ---: |
| 語&#10;次 | one&#10;two |
`;
  const article = renderArticleBody(prepareSource(source, defaultPreparationInputs()));
  expect(article.title).toBe("題名");
  expect(article.headings.map((heading) => heading.text)).toEqual([
    "節次",
    "英ASCII",
    "English words",
  ]);
  expect(elements(article.html, "th").map(textContent)).toEqual(["日本語", "English word"]);
  expect(elements(article.html, "td").map(textContent)).toEqual(["語次", "one two"]);
  expect(elements(article.html, "table")).toHaveLength(1);
  expect(elements(article.html, "tr")).toHaveLength(2);
  expect(elements(article.html, "th")[1]?.properties.dataAlign).toBe("right");
});

test("the renderer applies the same policy to all caption inputs without changing media references or alternative text", () => {
  const inputs = defaultPreparationInputs();
  inputs.images["image.png"] = { kind: "reference", value: { reference: "/assets/image.png?q=1" } };
  inputs.videos["clip.mp4"] = { kind: "reference", value: { reference: "/assets/clip.mp4?q=1" } };
  const source = `# 題

::image{src="image.png" alt="代替 テキスト" caption="画像"}

::video{src="clip.mp4" caption="動画"}

:::svg{alt="図" caption="SVG"}
<svg viewBox="0 0 10 10"><rect width="10" height="10" fill="currentColor" /></svg>
:::
`;
  const document = prepareSource(source, inputs);
  // 属性記法では改行を書けないため、公開APIへ渡す描画用文書のcaptionを直接指定する。
  for (const { content } of document) {
    if (content.kind !== "block") continue;
    const block = content.value;
    if (block.kind === "image" || block.kind === "video" || block.kind === "svg") {
      block.value.caption = "日本語。 \n English\n次。\n続き。 語間 空白";
    }
  }
  const original = JSON.stringify(document);
  const article = renderArticleBody(document);
  expect(elements(article.html, "figcaption").map(textContent)).toEqual([
    "日本語。English次。続き。 語間 空白",
    "日本語。English次。続き。 語間 空白",
    "日本語。English次。続き。 語間 空白",
  ]);
  expect(elements(article.html, "img")[0]?.properties.src).toBe("/assets/image.png?q=1");
  expect(elements(article.html, "img")[0]?.properties.alt).toBe("代替 テキスト");
  expect(elements(article.html, "video")[0]?.properties.src).toBe("/assets/clip.mp4?q=1");
  expect(JSON.stringify(document)).toBe(original);
});
