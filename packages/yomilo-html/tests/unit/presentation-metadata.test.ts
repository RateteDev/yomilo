import { expect, test } from "bun:test";
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs } from "@yomilo/wasm";
import { toHtml } from "hast-util-to-html";
import { buildTweetCard } from "../../src/tweet-card-build.js";
import { prepareSource } from "../utils/prepare.js";

test("rendering preserves media and alignment information without enforcing presentation", () => {
  const inputs = defaultPreparationInputs();
  inputs.images["photo.png"] = { kind: "reference", value: { reference: "photo.png" } };
  inputs.videos["clip.mp4"] = { kind: "reference", value: { reference: "clip.mp4" } };
  inputs.urlCards["https://example.com/"] = {
    title: { state: "available", value: "Example" },
    icon: {
      state: "available",
      value: { kind: "reference", value: { reference: "favicon.png" } },
    },
  };
  const source = `# Example

| Left | Center | Right |
| :--- | :---: | ---: |
| one | two | three |

::image{src="photo.png" alt="Photo" ratio="11/8"}

::video{src="clip.mp4" ratio="16/9"}

::url-card{href="https://example.com/"}

:::svg{alt="Diagram"}
<svg viewBox="0 0 10 10"><rect width="8" height="6" fill="currentColor" /></svg>
:::
`;
  const html = renderArticleBody(prepareSource(source, inputs)).html;
  expect(html).not.toMatch(/<(?:img|video|th|td)\b[^>]*\s(?:width|height|style|align)=/);
  expect(html).toContain('data-ratio-width="11" data-ratio-height="8"');
  expect(html).toContain('data-ratio-width="16" data-ratio-height="9"');
  expect(html).toContain('data-align="center"');
  expect(html).toContain('data-align="right"');
  expect(html).toContain('viewBox="0 0 10 10"');
  expect(html).toContain('width="8" height="6"');

  const tweet = toHtml(
    buildTweetCard({
      id: "1",
      text: "Photo",
      authorName: "Author",
      authorHandle: "author",
      authorAvatarUrl: "avatar.png",
      isVerified: true,
      createdAt: "2026-10-14",
      favoriteCount: 1,
      conversationCount: 2,
      inReplyToHandle: null,
      media: [{ type: "video", url: "photo.png", width: 1200, height: 800, altText: "Photo" }],
      quotedTweet: null,
      permalink: "https://x.com/author/status/1",
    }),
  );
  expect(tweet).not.toMatch(/<(?:img|svg)\b[^>]*\s(?:width|height|style)=/);
  expect(tweet).toContain('data-intrinsic-width="1200" data-intrinsic-height="800"');
});
