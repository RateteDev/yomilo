# WorkersとブラウザでのWASM

## Workers

wrangler（Cloudflare Workers）では、既定の入口がWASMをモジュールとして読み込むため、READMEの使い方がそのまま動きます。

## ブラウザ

`@yomilo/wasm/web`の`createYomilo`へ、WASMの本体を渡して初期化します。

```ts
import { createYomilo } from "@yomilo/wasm/web";

// WASMの本体（@yomilo/wasm/yomilo_wasm_bg.wasm）を、利用側が置いた場所から取得する
const wasm = await fetch("/yomilo_wasm_bg.wasm").then((r) => r.arrayBuffer());
// 既定の入口と同じ関数を持つオブジェクトを得る
const yomilo = createYomilo(wasm);
const { segments } = yomilo.segment(source, null, "complete");
```
