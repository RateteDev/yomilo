<p align="center">
  <br>
  <br>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../../assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="../../assets/logo-light.svg">
    <img src="../../assets/logo-light.svg" alt="Yomilo logo" width="320">
  </picture>
  <br>
</p>
<p align="center">
  <a href="https://github.com/RateteDev/yomilo/releases/latest"><img src="https://img.shields.io/github/v/release/RateteDev/yomilo?label=release&color=0969DA" alt="Latest release"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-MIT-2EA44F" alt="License: MIT"></a>
</p>
<p align="center">
  <a href="../../README.md">日本語</a> ・ English
</p>
<br/>

# Yomilo

Structure strict Markdown into a document model and render it as readable HTML.

- 🧱 Checks the source element by element and rejects only the elements that break the rules
- ⚡ Renders confirmed elements one by one from a draft that is still being written
- 🧩 Callouts, code groups, figures, images, videos, and link cards as directives
- 🎨 Body HTML and themes are distributed separately
- 🦀 A Rust core compiled to WASM, usable from Node, Bun, Workers, and browsers

Yomilo started as a way to display AI agents' writing legibly, but it is not limited to agents.

## Installation

Packages are distributed as tarballs on [GitHub Releases](https://github.com/RateteDev/yomilo/releases), not on npm.

```sh
# Install the WASM core and the HTML renderer (match the latest release; v0.1.0 is an example)
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-wasm-0.1.0.tgz https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-html-0.1.0.tgz
# Optionally install the minimal theme CSS
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-theme-0.1.0.tgz
```

## Usage

```ts
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs, prepareSegment, segment } from "@yomilo/wasm";

const source = await Bun.file("article.md").text();
// Split the source into one segment per top-level element
const { segments } = segment(source, null, "complete");
// Prepare with the default inputs, without fetched images or cards
const inputs = defaultPreparationInputs();
// Get the body HTML, title, headings, and diagnostics
const { html, title, diagnostics } = renderArticleBody(segments.map((s) => prepareSegment(s, inputs)));
```

`@yomilo/html` ships TypeScript sources, so load it from an environment that handles TypeScript, such as Bun or a bundler.
It has no CSS, scripts, table of contents, or page frame, so the consumer adds the look and interactions.

## Workers and browsers

The default entry of `@yomilo/wasm` reads the WASM file with Node APIs.
In other environments, pass the WASM binary to `createYomilo` from `@yomilo/wasm/web`.

```ts
import { createYomilo } from "@yomilo/wasm/web";
// On Workers, the WASM binary can be imported as a module
import wasm from "@yomilo/wasm/yomilo_wasm_bg.wasm";

// Get an object with the same functions as the default entry
const yomilo = createYomilo(wasm);
const { segments } = yomilo.segment(source, null, "complete");
```

## Theme

`@yomilo/theme/minimal.css` is a minimal theme that applies directly to the body HTML.
It uses the system fonts and bundles no font files.

## Notes

- Write the source in strict Markdown.
  Footnotes, strikethrough, task lists, Markdown image syntax, and raw HTML are not accepted.
- An element that breaks the rules comes back as a rejected segment with its source and diagnostics, and the other elements still render.
- `@yomilo/html` cannot be run directly from plain Node.

## License

[MIT License](../../LICENSE)
