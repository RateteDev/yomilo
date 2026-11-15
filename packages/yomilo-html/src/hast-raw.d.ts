import type { Literal } from "unist";

// エンジンが安全性を検査済みのSVGだけを、hast-util-to-htmlのallowDangerousHtmlへ渡す。
declare module "hast" {
  interface Raw extends Literal {
    type: "raw";
  }
  interface ElementContentMap {
    raw: Raw;
  }
  interface RootContentMap {
    raw: Raw;
  }
}
