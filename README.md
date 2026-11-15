<p align="center">
  <br>
  <br>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
    <img src="assets/logo-light.svg" alt="Yomilo のロゴ" width="320">
  </picture>
  <br>
</p>
<p align="center">
  <a href="https://github.com/RateteDev/yomilo/releases/latest"><img src="https://img.shields.io/github/v/release/RateteDev/yomilo?label=release&color=0969DA" alt="最新のリリース"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-2EA44F" alt="ライセンス: MIT"></a>
</p>
<p align="center">
  日本語 ・ <a href="docs/readme/README.en.md">English</a>
</p>
<br/>

# Yomilo

AIエージェントの出力や記事を、読みやすいHTMLにする部品です。
書きかけの原稿でも、確定した要素から順に描けます。

- 🧱 乱れた要素だけを取り分け、残りは描き続ける
- ⚡ 生成途中の原稿を、確定した要素から順に表示する
- 🧩 吹き出し・コードの切替・図・画像・動画・リンクカードを記法で書ける
- 🎨 本文のHTMLとテーマを分けて配る
- 🦀 Rustの中核をWASMで、Node・Bun・Workers・ブラウザへ

## 例

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/example-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/example-light.png">
    <img src="assets/example-light.png" alt="左に厳格Markdownの原稿、右に描画結果。取り消し線の1行だけが原文のまま取り分けられ、ほかは整って描かれている" width="860">
  </picture>
</p>

## 仕組み

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/how-it-works-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/how-it-works-light.svg">
    <img src="assets/how-it-works-light.svg" alt="原稿を判定し、描画準備を経て、描画でHTMLになる流れ" width="860">
  </picture>
</p>

語彙の定義は[用語集](docs/用語集.md)にあります。

## インストール

パッケージは[GitHub Releases](https://github.com/RateteDev/yomilo/releases)のtarballで配っています。

```sh
# 中核のWASMとHTMLの描画部を導入する（版はReleasesの最新に合わせる。例はv0.1.0）
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-wasm-0.1.0.tgz https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-html-0.1.0.tgz
# 最小テーマのCSSを使うときに導入する
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-theme-0.1.0.tgz
```

## 使い方

```ts
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs, prepareSegment, segment } from "@yomilo/wasm";

const source = await Bun.file("article.md").text();
// 判定
const { segments } = segment(source, null, "complete");
// 描画準備（取得結果が空の既定の入力）
const inputs = defaultPreparationInputs();
// 描画
const { html, title, diagnostics } = renderArticleBody(segments.map((s) => prepareSegment(s, inputs)));
```

`@yomilo/html`はTypeScriptのソースで配るため、BunやバンドラーなどTypeScriptを扱える環境で使います。
WorkersとブラウザでのWASMの読み込みは[別の文書](docs/WorkersとブラウザでのWASM.md)にあります。

## テーマ

`@yomilo/theme/minimal.css`を本文のHTMLへ当てると、ライトとダークで読みやすく表示されます。

## 記法

- 厳格なMarkdown（見出し・段落・リスト・引用・表・コードなど）を使えます。
- 吹き出し・コードの切替・図・画像・動画・リンクカードは、ディレクティブで書きます。
- 脚注・取り消し線・タスクリスト・生のHTMLを含む要素は、拒否されたセグメントとして原文と診断が返ります。

## ライセンス

[MIT License](LICENSE)
