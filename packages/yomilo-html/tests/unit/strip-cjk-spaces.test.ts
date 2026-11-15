import { expect, test } from "bun:test";
import { stripCjkSpaces } from "../../src/strip-cjk-spaces.js";

test("single-line directive attributes remain literal while body spacing is removed", () => {
  const directive =
    '::image{src="図 1.png" alt="日本語 `API` と SVG" ratio="1/1" caption=\'https://example.com/図\'}';
  expect(stripCjkSpaces(`${directive}\n\n本文 TypeScript と API です。`)).toBe(
    `${directive}\n\n本文TypeScriptとAPIです。`,
  );
});

test("block directive attributes preserve quoted braces and escapes without protecting the body", () => {
  const directive = ':::callout{type=note label="注意 } \\"日本語 API\\" $& \'値 2\'"}';
  expect(stripCjkSpaces(`${directive}\n本文 TypeScript と SVG です。\n:::`)).toBe(
    `${directive}\n本文TypeScriptとSVGです。\n:::`,
  );
});
