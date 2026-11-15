import type { Element } from "hast";
import { fromHtmlIsomorphic } from "hast-util-from-html-isomorphic";
import { MOTION_ELEMENTS } from "./svg-motion.js";

const COLOR_ATTRIBUTES = new Set([
  "fill",
  "stroke",
  "color",
  "stop-color",
  "flood-color",
  "lighting-color",
]);
const COLOR_PROPERTIES = new Set(
  [...COLOR_ATTRIBUTES].map((name) =>
    name.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()),
  ),
);
const THEME_COLOR =
  /^(?:var\(--[a-z0-9-]+\)|currentColor|none|transparent|inherit|url\(#[^)\s]+\))$/;
const ANIMATABLE_ATTRIBUTES = new Set([
  "opacity",
  "fill-opacity",
  "stroke-opacity",
  "stop-opacity",
  "stroke-width",
  "stroke-dasharray",
  "stroke-dashoffset",
  "offset",
  "x",
  "y",
  "x1",
  "y1",
  "x2",
  "y2",
  "cx",
  "cy",
  "r",
  "rx",
  "ry",
  "width",
  "height",
  "d",
  "points",
  "transform",
  "visibility",
]);

// 実行要素・外部参照・IDの隔離はyomiloが検査済み。ここでは記事のテーマと寸法の制約だけを追加する。
export function assertEngineSvgPresentation(source: string): void {
  const tree = fromHtmlIsomorphic(source, { fragment: true });
  const roots = tree.children.filter((node) => node.type === "element");
  const root = roots[0];
  if (
    roots.length !== 1 ||
    root?.tagName !== "svg" ||
    !root.properties.viewBox ||
    root.properties.width !== undefined ||
    root.properties.height !== undefined
  ) {
    throw new Error(
      "SvgInvalidFrame: SVG needs one root with a viewBox and no fixed width or height.",
    );
  }
  const visit = (node: Element): void => {
    for (const [name, value] of Object.entries(node.properties)) {
      if (name === "className" || name === "style") {
        throw new Error(`SvgForbiddenContent: SVG presentation must not include ${name}.`);
      }
      if (
        COLOR_PROPERTIES.has(name) &&
        (typeof value !== "string" || !THEME_COLOR.test(value.trim()))
      ) {
        throw new Error(
          `SvgFixedColor: ${name} must use a theme token, currentColor, or a local paint reference.`,
        );
      }
    }
    if (
      MOTION_ELEMENTS.has(node.tagName) &&
      node.tagName !== "animateTransform" &&
      node.tagName !== "animateMotion"
    ) {
      const target = node.properties.attributeName;
      if (typeof target === "string" && COLOR_ATTRIBUTES.has(target)) {
        throw new Error(
          "SvgFixedColor: SVG must not animate a colour; animate geometry or opacity instead.",
        );
      }
      if (typeof target !== "string" || !ANIMATABLE_ATTRIBUTES.has(target)) {
        throw new Error(
          "SvgForbiddenContent: SVG may animate only geometry, opacity, or transform.",
        );
      }
    }
    for (const child of node.children) if (child.type === "element") visit(child);
  };
  visit(root);
}
