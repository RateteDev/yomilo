# Yomilo

**Yomilo（ヨミロ）** は、厳格なMarkdownを構造化し、読みやすく美しい表示へつなぐための独立した文書エンジンです。
AIエージェントの文章を読みやすく表示したいという動機から生まれましたが、エージェント専用の設計ではありません。

> **状態：ローカル開発・未公開。
> ** Rustによる構造化・描画準備、Node/Bun向けWASM bridgeとTSのHTML Rendererを実装しています。
> npm公開・ネイティブFFIはまだ行っていません。
> 残っている作業はコード中の`TODO(名前)`で管理しています。

## 設計資料

- [用語集](docs/用語集.md) — 文書表現と処理責務の定義
- [ADR](docs/adr/) — 判断の記録

処理の基本経路：**厳格Markdownソース → 構造化 → 構造化文書 → 描画準備 → 描画用文書 → 描画 → 描画結果**。

当面は生成側が構造化・描画準備を担当します。
構造化と描画準備は論理上は別責務ですが、現在はどちらもRustのCore crateに実装しています。
HTML描画は別のTSパッケージで、ページ・URL・配色は利用側が決めます。

## このディレクトリの扱い

- `crates/yomilo-core/` — 構造化、入力診断、正規化、描画準備と公開文書モデル
- `crates/yomilo-wasm/` — 非公開の`@yomilo/wasm`。
  Rustからの型とNode/Bun向けWASMを供給
- `packages/yomilo-html/` — 非公開の`@yomilo/html`。
  本文のHTMLとフック、frontmatterと入力の検査、カードの取得
- `packages/yomilo-theme/` — 非公開の`@yomilo/theme`。
  本文HTMLへそのまま当てる最小テーマのCSS`minimal.css`
- `docs/` — 用語集・ADR・資産の取り扱い
- `experiments/phase0-rust/` — 成立性検証のソース。
  **製品コードではありません**
- `examples/` — 今後のサンプル用

## ライセンス

新規に作成するYomiloのコードは[MIT](LICENSE)です。
参照用の既存資産や第三者フォントへ自動的にMITを適用するものではありません。
詳細は[資産の取り扱い](docs/資産の取り扱い.md)を参照してください。

## 開発・公開

Gitで管理していますが、リモートはなく、パッケージも公開していません。
Rust workspaceとBun workspaceは同居し、版とlockfileはそれぞれ持ちます。

```sh
just setup
just wasm-build   # 初回、またはRustの契約・実装を変更したとき
just core-ci      # Rustだけ。Bunは不要
just html-ci      # 準備済みWASMを使い、内部でRustを起動しない
just ci-local
```

ツール版は`mise.toml`を正とします。
WASMと型の準備は明示的に行います。
CI設定はRust検査とWASM生成を分け、HTML検査へWASMを渡しますが、リモートでの実行は未確認です。

HTMLパッケージのexportsはTSソースです。
BunとAstro等のTSを処理する利用側を対象とし、素のNodeからの直接実行は未対応です。
CSS・操作スクリプト・アイコン・ページの枠・目次・テーマの切替は持たず、利用側が用意します。

本文のHTMLはアイコンへ表示寸法を指定せず、メディアの比率は`data-ratio-width`と`data-ratio-height`、素材の寸法は`data-intrinsic-width`と`data-intrinsic-height`、表の列揃えは`data-align`へ保持します。
表示へ反映するか・どう反映するかはテーマが決め、原稿内SVGの座標と図形は原稿の内容として保持します。

`/engine`の`renderArticleBody`は本文のHTMLとフックだけを返し、CSS・JS・テーマを受け取りません。
操作子と装飾アイコンを出さず本文を隠さないため、開閉の初期状態は`data-fold`、図の拡大の指定は`data-zoom`、カードの取得状態は`data-icon-state`へ残し、見せ方と操作は利用側が付けます。
