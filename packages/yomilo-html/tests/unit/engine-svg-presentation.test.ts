import { expect, test } from "bun:test";
import { assertEngineSvgPresentation } from "../../src/engine-svg-presentation.js";

test("HTML SVG presentation refuses fixed paint and dimensions after engine safety checks", () => {
  expect(() =>
    assertEngineSvgPresentation(
      '<svg viewBox="0 0 10 10"><text fill="currentColor">a *b*</text></svg>',
    ),
  ).not.toThrow();
  expect(() => assertEngineSvgPresentation('<svg viewBox="0 0 10 10" width="10"></svg>')).toThrow(
    "SvgInvalidFrame",
  );
  expect(() =>
    assertEngineSvgPresentation('<svg viewBox="0 0 10 10"><rect fill="#112233" /></svg>'),
  ).toThrow("SvgFixedColor");
  expect(() =>
    assertEngineSvgPresentation(
      '<svg viewBox="0 0 10 10"><rect fill="var(--ink)"><animate attributeName="fill" values="var(--ink);red" /></rect></svg>',
    ),
  ).toThrow("SvgFixedColor");
});
