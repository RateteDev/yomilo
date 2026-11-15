---
version: 99
custom: [preserve, opaque]
---

# 適合カタログ

段落では*強調*、**強い強調**、`inline_code`、[明示リンク](https://example.com/article)を保持します。

## 共通構文

### > 閉じた小節

閉じた小節の本文。

### v 開いた小節

開いた小節の本文。

### 通常の小節

通常の小節の本文。

- 箇条書き一
- 箇条書き二

1. 番号一
2. 番号二

> 引用本文。
>
> - 引用内の項目

```rust
fn main() {}
```

| 指定なし | 左 | 中央 | 右 |
| --- | :--- | :---: | ---: |
| 無指定 | 左寄せ | 中央寄せ | 右寄せ |

---

## ディレクティブ

:::callout{type="note" label="NOTE"}
既定noteの本文。
:::

:::callout{type="note" label="補足 ラベル"}
独自noteの本文。
:::

:::callout{type="warning" label="WARNING"}
既定warningの本文。
:::

:::callout{type="warning" label="警告 ラベル"}
独自warningの本文。
:::

:::callout{type="caution" label="CAUTION"}
既定cautionの本文。
:::

:::callout{type="caution" label="注意 ラベル"}
独自cautionの本文。
:::

:::code-group
```bash npm
npm install package
```

```bash pnpm
pnpm add package
```
:::

:::svg{alt="静的 図" caption="SVGの 説明" zoom}
<svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>
:::

::image{src="figure.png" alt="画像の 代替説明" ratio="16/9" caption="画像の 説明" zoom}

::video{src="clip.mp4" ratio="4/3" caption="動画の 説明" loop}

::url-card{href="https://example.com/article"}

::tweet-card{href="https://x.com/example/status/1"}
