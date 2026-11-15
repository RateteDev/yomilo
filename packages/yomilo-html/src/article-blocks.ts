import type { Element, ElementContent } from "hast";

export interface CalloutOptions {
  type: string;
  labelText: string;
}

export function buildCallout(options: CalloutOptions, children: ElementContent[]): Element {
  return {
    type: "element",
    tagName: "div",
    properties: { class: `callout ${options.type}` },
    children: [
      {
        type: "element",
        tagName: "span",
        properties: { class: "callout-label" },
        children: [{ type: "text", value: options.labelText }],
      },
      {
        type: "element",
        tagName: "div",
        properties: { class: "callout-body" },
        children,
      },
    ],
  };
}
