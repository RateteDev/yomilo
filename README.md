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

厳格なMarkdownを文書モデルへ構造化し、読みやすいHTMLへ描く。

- 🧱 原稿を要素ごとに判定し、規約に反した要素だけを拒否
- ⚡ 書きかけの原稿から、確定した要素を順に描画
- 🧩 吹き出し・コードの切替・図・画像・動画・リンクカードを記法で記述
- 🎨 本文のHTMLとテーマを分けて配布
- 🦀 Rustの中核をWASMで、Node・Bun・Workers・ブラウザから利用

AIエージェントの文章を読みやすく表示したいという動機から生まれましたが、用途はエージェントに限りません。

## インストール

パッケージはnpmではなく、[GitHub Releases](https://github.com/RateteDev/yomilo/releases)のtarballで配っています。

```sh
# 中核のWASMとHTMLの描画部を導入する（版はReleasesの最新に合わせる。例はv0.1.0）
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-wasm-0.1.0.tgz https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-html-0.1.0.tgz
# 必要なら最小テーマのCSSも導入する
bun add https://github.com/RateteDev/yomilo/releases/download/v0.1.0/yomilo-theme-0.1.0.tgz
```

## 使い方

```ts
import { renderArticleBody } from "@yomilo/html/engine";
import { defaultPreparationInputs, prepareSegment, segment } from "@yomilo/wasm";

const source = await Bun.file("article.md").text();
// 原稿を要素ごとのセグメントに分ける
const { segments } = segment(source, null, "complete");
// 画像やカードの取得結果を渡さず、既定の入力で描画準備する
const inputs = defaultPreparationInputs();
// 本文のHTMLと題・見出し・診断を得る
const { html, title, diagnostics } = renderArticleBody(segments.map((s) => prepareSegment(s, inputs)));
```

`@yomilo/html`はTypeScriptのソースを配っており、BunやバンドラーなどTypeScriptを扱える環境から読み込みます。
CSS・操作スクリプト・目次・ページの枠は持たないため、見た目と操作は利用側で付けます。

## Workersとブラウザ

Workers（wrangler）では、既定の入口がWASMをモジュールとして読み込むため、そのまま使えます。
ブラウザなどでは`@yomilo/wasm/web`の`createYomilo`へ、WASMの本体を渡します。

```ts
import { createYomilo } from "@yomilo/wasm/web";

// WASMの本体（@yomilo/wasm/yomilo_wasm_bg.wasm）を、利用側が置いた場所から取得する
const wasm = await fetch("/yomilo_wasm_bg.wasm").then((r) => r.arrayBuffer());
// 既定の入口と同じ関数を持つオブジェクトを得る
const yomilo = createYomilo(wasm);
const { segments } = yomilo.segment(source, null, "complete");
```

## テーマ

`@yomilo/theme/minimal.css`は、本文のHTMLへそのまま当てられる最小のテーマです。
環境の書体を使い、フォントの本体は含みません。

## 注意点

- 原稿は厳格なMarkdownで書きます。
  脚注・取り消し線・タスクリスト・Markdownの画像記法・生のHTMLは受け付けません。
- 規約に反した要素は、拒否されたセグメントとして原文と診断を返し、ほかの要素の描画は続けます。
- 素のNodeから`@yomilo/html`を直接実行することはできません。

## ライセンス

[MIT License](LICENSE)
