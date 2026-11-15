import { afterAll, beforeAll, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type MediaResolveOptions, resolveMedia } from "@yomilo/html";
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs, type PreparationInputs } from "@yomilo/wasm";
import { fromHtmlIsomorphic } from "hast-util-from-html-isomorphic";
import { toHtml } from "hast-util-to-html";
import { prepareSource } from "../utils/prepare.js";

async function render(source: string, media: MediaResolveOptions, inputs: PreparationInputs) {
  const article = renderArticleBody(prepareSource(source, inputs));
  const tree = fromHtmlIsomorphic(article.html, { fragment: true });
  await resolveMedia(tree, media);
  return { html: toHtml(tree) };
}

function imageInput(source: string) {
  const inputs = defaultPreparationInputs();
  inputs.images[source] = { kind: "reference", value: { reference: source } };
  return inputs;
}

const PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6p1sAAAAASUVORK5CYII=";
// fixtureはffmpegで生成した16×16の黒い1フレーム。テスト実行にffmpegは要らない。
const VIDEO = Buffer.from(
  readFileSync(new URL("../utils/clip.mp4.base64", import.meta.url), "utf-8"),
  "base64",
);
let directory: string;
let server: ReturnType<typeof Bun.serve>;

beforeAll(() => {
  directory = mkdtempSync(join(tmpdir(), "test-article-media-"));
  writeFileSync(join(directory, "image.png"), Buffer.from(PNG, "base64"));
  writeFileSync(join(directory, "clip.mp4"), VIDEO);
  server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    fetch: () => new Response(Bun.file(join(directory, "image.png"))),
  });
  expect(server.hostname).toBe("127.0.0.1");
});

afterAll(() => {
  server?.stop(true);
  if (directory) rmSync(directory, { recursive: true, force: true });
});

test("embeds local and HTTP media from real files, preserving embedded data and response MIME", async () => {
  const remote = `${server.url}without-extension`;
  const inputs = imageInput("image.png");
  inputs.images[remote] = { kind: "reference", value: { reference: remote } };
  inputs.images["embedded.png"] = {
    kind: "embedded",
    value: { mediaType: "image/png", bytes: PNG },
  };
  inputs.videos["clip.mp4"] = { kind: "reference", value: { reference: "clip.mp4" } };
  const { html } = await render(
    `::image{src="image.png"}\n\n::image{src="${remote}" alt="図"}\n\n::image{src="embedded.png"}\n\n::video{src="clip.mp4"}\n`,
    { mode: "inline", baseDir: directory, warn: () => undefined },
    inputs,
  );
  expect([...html.matchAll(/src="([^"]+)"/g)].map((match) => match[1])).toEqual([
    `data:image/png;base64,${PNG}`,
    `data:image/png;base64,${PNG}`,
    `data:image/png;base64,${PNG}`,
    `data:video/mp4;base64,${VIDEO.toString("base64")}`,
  ]);
});

test("link mode preserves absolute URLs but local and missing references stop output", async () => {
  const media = { mode: "link" as const, baseDir: directory, warn: () => undefined };
  const remote = `${server.url}image.png`;
  const { html } = await render(`::image{src="${remote}" alt="図"}\n`, media, imageInput(remote));
  expect(html).toContain(`src="${server.url}image.png"`);
  await expect(
    render('::image{src="image.png" alt="図"}\n', media, imageInput("image.png")),
  ).rejects.toThrow("image.png");
  await expect(
    render(
      '::image{src="missing.png" alt="図"}\n',
      { ...media, mode: "inline" },
      imageInput("missing.png"),
    ),
  ).rejects.toThrow("missing.png");
});

test("embeds the avatars and photos of a tweet card, and leaves the card's own link alone", async () => {
  const remote = (name: string) => `${server.url}${name}`;
  const permalink = "https://x.com/handle/status/1";
  const reference = (name: string) => ({
    kind: "reference" as const,
    value: { reference: remote(name) },
  });
  const media = { mode: "inline" as const, baseDir: directory, warn: () => undefined };
  const source = '::tweet-card{href="https://x.com/handle/status/1"}\n';
  const inputs = defaultPreparationInputs();
  inputs.tweetCards[permalink] = {
    state: "available",
    value: {
      id: "1",
      author: {
        name: "名前",
        handle: "handle",
        profileHref: "https://x.com/handle",
        avatar: reference("avatar.png"),
      },
      text: "本文",
      createdAt: { kind: "original", value: "2026-10-11T00:00:00.000Z" },
      counts: { replies: 0, reposts: null, likes: 0, quotes: null, views: null, bookmarks: null },
      links: [],
      media: [
        {
          kind: "image",
          delivery: reference("photo.png"),
          alt: null,
          dimensions: { width: 1, height: 1 },
          poster: null,
        },
      ],
    },
  };

  const { html } = await render(source, media, inputs);
  expect([...html.matchAll(/<img[^>]*\ssrc="([^"]+)"/g)].map((match) => match[1])).toEqual([
    `data:image/png;base64,${PNG}`,
    `data:image/png;base64,${PNG}`,
  ]);
  expect(html).toContain('href="https://x.com/handle/status/1"');

  const link = await render(source, { ...media, mode: "link" }, inputs);
  expect(link.html).toContain(`src="${remote("avatar.png")}"`);
});
