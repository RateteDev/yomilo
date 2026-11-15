# 記事描画エンジン Rust 採用検証 — 2026-10-12

`own/web/ROADMAP-記事描画エンジン.md` の Phase 0 に向けた**使い捨ての技術検証を再現するための小規模な保存版**。製品コードや正式な仕様ではない。

## 構成

- `native/`: `markdown-rs`、`two-face` / `syntect`、`budouy` のRust検証。`src/bin/directives.rs`は最小限のディレクティブ分離検証。
- `wasm/`: `wasm-bindgen`でRustのMarkdown解析・日本語分割・構文スコープ取り出しをNode/Bunに公開する検証。
- `astro/`: WASMから得た構造化結果をAstro静的ページに差し込む最小検証。`compiled-sample.html`は生成されたHTMLの例。

## 確認された結果

- Markdownのfrontmatter、見出し、段落、リスト、表、コードフェンスが`markdown-rs`で解析できた。既存MDXカタログの構文木への変換も実行成功。ただし**現行remarkとの構造の完全一致は未検証**。
- ブロックディレクティブをMarkdownのパース**前**に切り出し、Callout内の箇条書きを通常のMarkdownとしてパースできた。コードフェンス中の `:::` を閉じ記号に誤認識しない簡易例も成功。**属性の完全な解析・位置情報・ネスト・正規形出力は未対応**。
- `two-face` / `syntect`でTSの構文スコープ取り出しがNative・WASM両方で成功。`jsx` / `jsonc`という文字列は今回の文法探索では未解決。全対応言語や色分けの品質は未確認。
- `budouy`は動作したが、JS版BudouXとの5例比較では**4例一致、1例不一致**（「設計方針」がRust側で分割された）。同等性は未確定。
- Rust → WASM → Node.js/Bun の実行に成功。Astro 6.4.2の最小静的ページへつなぎ、HTML生成に成功。**既存サイトへの統合、実際のHTMLデザイン・CSSの一致は未検証**。
- WASMサイズは検証内容・文法セットで変化した。簡易のMarkdown+BudouX構成は約610KB、syntect文法を含む構成は約3.2MB（どちらも未最適化の検証ビルド）。最終配布サイズではない。

## 再実行

通常のRust環境で:

```sh
cd native
cargo run --bin article-engine-poc -- <既存記事.mdx>
cargo run --bin directives
```

WASM:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd wasm
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target nodejs --out-dir pkg target/wasm32-unknown-unknown/release/article_wasm_poc.wasm
node probe.cjs
bun probe.cjs
```

Astroは既存 `own/web` の Astro 6.4.2 の依存環境を検証に使用した。再現する場合は、`astro/`に対応バージョンのAstro依存関係を用意してから、`ARTICLE_POC_WASM_JS`に**生成した `wasm/pkg/article_wasm_poc.js` の絶対パス**を指定し、`astro build`を実行する。`astro/src/pages/index.astro`は最小限のHTMLのみ生成する。

`native`は、既存記事として解析を試すMDXのパスを最初の引数に取る。

## 残課題

正式なディレクティブ文法の仕様、正規形の書き戻し、型とJSON契約、セキュアなHTML生成、見た目のパリティ、CLI単体配布、WASMのバンドル・サイズ最適化、ネイティブとJSの文節分割差分を確認する必要がある。

PoCを正式なライブラリへ昇格させない。新エンジンの実装時は、ここで分かった問題と判断を仕様・契約テストへ反映する。
