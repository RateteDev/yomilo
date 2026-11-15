import type { Element, Parent, Root } from "hast";
import { phraseWrap } from "./phrase-wrap.js";

function hasTableWrapClass(node: Element): boolean {
  const className: unknown = node.properties?.className;
  if (Array.isArray(className)) {
    return className.includes("table-wrap");
  }
  if (typeof className === "string") {
    return className.split(/\s+/).includes("table-wrap");
  }
  return false;
}

function transformTableChildren(parent: Parent): void {
  for (let index = 0; index < parent.children.length; index += 1) {
    const child = parent.children[index];
    if (child?.type !== "element") {
      continue;
    }
    if (child.tagName === "table") {
      if (
        parent.type === "element" &&
        (parent as Element).tagName === "div" &&
        hasTableWrapClass(parent as Element)
      ) {
        continue;
      }
      const wrapped: Element = {
        type: "element",
        tagName: "div",
        properties: { className: ["table-wrap"] },
        children: [child],
      };
      parent.children[index] = wrapped;
      continue;
    }
    transformTableChildren(child);
  }
}

export function rehypeTableWrap() {
  return (tree: Root): void => {
    transformTableChildren(tree);
  };
}

function phraseWrapTextNodes(parent: Parent): void {
  for (const child of parent.children) {
    if (child.type === "text") {
      child.value = phraseWrap(child.value);
      continue;
    }
    if (child.type === "element" && child.tagName !== "code") {
      phraseWrapTextNodes(child);
    }
  }
}

function transformHeadingWrapPoints(parent: Parent): void {
  for (const child of parent.children) {
    if (child.type !== "element") {
      continue;
    }
    if (child.tagName === "h2") {
      phraseWrapTextNodes(child);
      continue;
    }
    transformHeadingWrapPoints(child);
  }
}

/**
 * Mark phrase boundaries in h2 text with zero-width spaces so a heading that
 * does not fit breaks between phrases instead of mid-word. Inline code keeps
 * its text untouched. The CSS side must pair this with `word-break: keep-all`.
 */
export function rehypeHeadingPhraseWrap() {
  return (tree: Root): void => {
    transformHeadingWrapPoints(tree);
  };
}
