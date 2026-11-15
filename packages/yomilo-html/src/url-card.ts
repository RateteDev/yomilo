import type { Element, ElementContent } from "hast";
import type { OgpData } from "./url-card-ogp.js";

export type FetchOgp = (url: string) => Promise<OgpData>;

/** `favicon` is the image's `src` as it is written into the card. */
export function buildUrlCard(
  url: string,
  ogp: { title: string | null; favicon: string | null },
): Element {
  const displayUrl = (() => {
    try {
      const parsed = new URL(url);
      return `${parsed.hostname}${parsed.pathname}${parsed.search}${parsed.hash}`.replace(
        /\/$/,
        "",
      );
    } catch {
      return url;
    }
  })();

  const bodyChildren: ElementContent[] = [
    {
      type: "element",
      tagName: "span",
      properties: { className: ["url-card__title"] },
      children: [{ type: "text", value: ogp.title ?? displayUrl }],
    },
  ];

  bodyChildren.push({
    type: "element",
    tagName: "span",
    properties: { className: ["url-card__url"] },
    children: [{ type: "text", value: displayUrl }],
  });

  return {
    type: "element",
    tagName: "a",
    properties: {
      className: ["url-card"],
      href: url,
      target: "_blank",
      rel: ["noopener", "noreferrer"],
    },
    children: [
      ...(ogp.favicon
        ? [
            {
              type: "element" as const,
              tagName: "img",
              properties: { className: ["url-card__favicon"], src: ogp.favicon, alt: "" },
              children: [],
            },
          ]
        : []),
      {
        type: "element",
        tagName: "div",
        properties: { className: ["url-card__body"] },
        children: bodyChildren,
      },
    ],
  };
}
