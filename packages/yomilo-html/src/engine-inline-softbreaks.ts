import type { ElementContent, Text } from "hast";

const JAPANESE_CHARACTER = String.raw`[\p{Script_Extensions=Han}\p{Script_Extensions=Hiragana}\p{Script_Extensions=Katakana}\u3001-\u303f\u2014\u2015\u2025\u2026\uff01-\uff0f\uff1a-\uff20\uff3b-\uff40\uff5b-\uff60\uffe0-\uffe6]`;
const JAPANESE_END = new RegExp(`${JAPANESE_CHARACTER}$`, "u");
const JAPANESE_START = new RegExp(`^${JAPANESE_CHARACTER}`, "u");
const SOFTBREAK = /[ \t\r\f]*\n[ \t\r\f\n]*/g;

interface TextSpan {
  node: Text;
  start: number;
  end: number;
  protected: boolean;
}

interface TextEdit {
  start: number;
  end: number;
  replacement: string;
}

// TODO(engine-cjk-softbreak): 描画準備が言語別の軟改行方針を供給する段階で、この補正を置き換える。原稿の正規化とCJK空白の執筆検査は別の責務。
export function normalizeEngineInlineSoftbreaks(nodes: ElementContent[]): void {
  const spans: TextSpan[] = [];
  let source = "";
  const collect = (children: ElementContent[], protectedText = false): void => {
    for (const child of children) {
      if (child.type === "text") {
        const start = source.length;
        source += child.value;
        spans.push({ node: child, start, end: source.length, protected: protectedText });
      } else if (child.type === "element") {
        collect(child.children, protectedText || child.tagName === "code");
      }
    }
  };
  collect(nodes);

  const edits = new Map<Text, TextEdit[]>();
  let spanIndex = 0;
  for (const match of source.matchAll(SOFTBREAK)) {
    const start = match.index;
    const end = start + match[0].length;
    while (spanIndex < spans.length) {
      const span = spans[spanIndex];
      if (!span) throw new Error("Missing inline text span");
      if (span.end > start) break;
      spanIndex++;
    }
    const editable: { span: TextSpan; start: number; end: number }[] = [];
    for (let index = spanIndex; index < spans.length; index++) {
      const span = spans[index];
      if (!span || span.start >= end) break;
      if (!span.protected && span.end > span.start) {
        editable.push({ span, start: Math.max(start, span.start), end: Math.min(end, span.end) });
      }
    }
    // インラインコードの文字は境界判定に使うが、内部の空白・改行は変更しない。
    if (!editable.some((part) => source.slice(part.start, part.end).includes("\n"))) continue;
    const touchesJapanese =
      JAPANESE_END.test(source.slice(Math.max(0, start - 2), start)) ||
      JAPANESE_START.test(source.slice(end, end + 2));
    for (const [index, part] of editable.entries()) {
      const nodeEdits = edits.get(part.span.node) ?? [];
      nodeEdits.push({
        start: part.start - part.span.start,
        end: part.end - part.span.start,
        replacement: index === 0 && !touchesJapanese ? " " : "",
      });
      edits.set(part.span.node, nodeEdits);
    }
  }

  for (const [node, nodeEdits] of edits) {
    let cursor = 0;
    let value = "";
    for (const edit of nodeEdits) {
      value += node.value.slice(cursor, edit.start) + edit.replacement;
      cursor = edit.end;
    }
    node.value = value + node.value.slice(cursor);
  }
}
