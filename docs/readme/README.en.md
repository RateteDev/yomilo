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

Turns AI agent output and articles into readable HTML.
It renders a draft that is still being written, one confirmed element at a time.

- 🧱 Sets aside only the broken elements and keeps rendering the rest
- ⚡ Shows a draft in progress, one confirmed element at a time
- 🧩 Writes callouts, code groups, figures, images, videos, and link cards as directives
- 🎨 Ships the body HTML and the theme separately
- 🦀 A Rust core in WASM, for Node, Bun, Workers, and browsers

## Example

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../../assets/example-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="../../assets/example-light.png">
    <img src="../../assets/example-light.png" alt="Strict Markdown source on the left and the rendered result on the right. Only the strikethrough line is set aside as its original text, and the rest renders neatly" width="860">
  </picture>
</p>

## How it works

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../../assets/how-it-works-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="../../assets/how-it-works-light.svg">
    <img src="../../assets/how-it-works-light.svg" alt="The source is checked, prepared, and rendered into HTML" width="860">
  </picture>
</p>

The vocabulary is defined in the [glossary](../用語集.md) (Japanese).

## Installation

Packages are distributed as tarballs on [GitHub Releases](https://github.com/RateteDev/yomilo/releases).

```sh
# Install the WASM core and the HTML renderer (match the latest release; v0.1.0 is an example)
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-wasm-0.1.0.tgz https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-html-0.1.0.tgz
# Install the minimal theme CSS when you use it
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-theme-0.1.0.tgz
```

## Usage

```ts
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs, prepareSegment, segment } from "@yomilo/wasm";

const source = await Bun.file("article.md").text();
// Check
const { segments } = segment(source, null, "complete");
// Prepare (the default inputs, with empty fetched results)
const inputs = defaultPreparationInputs();
// Render
const { html, title, diagnostics } = renderArticleBody(segments.map((s) => prepareSegment(s, inputs)));
```

`@yomilo/html` ships TypeScript sources, so use it in an environment that handles TypeScript, such as Bun or a bundler.
Loading the WASM on Workers and in browsers is described in a [separate document](../WorkersとブラウザでのWASM.md) (Japanese).

## Theme

Apply `@yomilo/theme/minimal.css` to the body HTML for a readable display in light and dark.

## Syntax

- Strict Markdown (headings, paragraphs, lists, quotes, tables, code, and more) is available.
- Callouts, code groups, figures, images, videos, and link cards are written as directives.
- An element with a footnote, strikethrough, a task list, or raw HTML comes back as a rejected segment with its source and diagnostics.

## License

[MIT License](../../LICENSE)
