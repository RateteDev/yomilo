import type { Element, ElementContent, Root, RootContent } from "hast";

type CollapseState = "closed" | "open";

/**
 * 記事の構造を、idで指せる形に整える。
 *
 * 長い記録を開くエージェントへ、構成と読むべき箇所を案内する説明書から範囲を指させるため、
 * 見出しから次の同じ階層の見出しまでを`<section>`で囲み、見出しのidをその箱へ移す。idの
 * 指す要素を取り出せば本文まで過不足なく手に入るので、範囲の決まりを道具ごとに持たせずに
 * 済む。書き手が書いたコンポーネント・コードブロック・表にも、種類ごとの連番idを振る。
 *
 * どの部品も組み上がった後の木を受け取る。webとCLIは同じ木の形を作るため、この変換を
 * 最後に通せば同じidになる。
 */

/** 見出し以外の要素の番号の桁数。画面に番号を出さないため、長い記録でも足りる3桁とする。 */
export const ELEMENT_NUMBER_DIGITS = 3;

function classesOf(node: Element): string[] {
  const className: unknown = node.properties?.className;
  if (Array.isArray(className)) {
    return className.map(String);
  }
  if (typeof className === "string") {
    return className.split(/\s+/);
  }
  const raw = node.properties?.class;
  return typeof raw === "string" ? raw.split(/\s+/) : [];
}

function isElement(node: RootContent | ElementContent | undefined): node is Element {
  return node?.type === "element";
}

function sectionHeadingOf(node: RootContent | ElementContent): Element | null {
  return isElement(node) && node.tagName === "h2" ? node : null;
}

function takeId(heading: Element): string | null {
  const id = heading.properties?.id;
  if (typeof id !== "string" || id === "") {
    return null;
  }
  delete heading.properties.id;
  return id;
}

function takeCollapseState(heading: Element): CollapseState | null {
  const state = heading.properties?.dataCollapse;
  if (state !== "closed" && state !== "open") {
    return null;
  }
  delete heading.properties.dataCollapse;
  return state;
}

function section(id: string, first: RootContent | ElementContent): Element {
  return {
    type: "element",
    tagName: "section",
    properties: { id },
    children: [first as ElementContent],
  };
}

/**
 * 本文の直下を、セクションとサブセクションの箱へ組み替える。最初のセクションより前の
 * 導入文と、idを持たない見出しは、そのまま元の位置に残す。
 */
function wrapSections(
  children: (RootContent | ElementContent)[],
): (RootContent | ElementContent)[] {
  const result: (RootContent | ElementContent)[] = [];
  const collapsible = new Map<Element, CollapseState>();
  let current: Element | null = null;
  let currentSubsection: Element | null = null;

  for (const child of children) {
    const sectionHeading = sectionHeadingOf(child);
    const sectionId = sectionHeading ? takeId(sectionHeading) : null;
    if (sectionId !== null) {
      current = section(sectionId, child);
      currentSubsection = null;
      result.push(current);
      continue;
    }

    if (current !== null && isElement(child) && child.tagName === "h3") {
      const subsectionId = takeId(child);
      const state = takeCollapseState(child);
      if (subsectionId !== null) {
        currentSubsection = section(subsectionId, child);
        current.children.push(currentSubsection);
        if (state !== null) {
          collapsible.set(currentSubsection, state);
        }
        continue;
      }
    }

    const container = currentSubsection ?? current;
    if (container === null) {
      result.push(child);
    } else {
      container.children.push(child as ElementContent);
    }
  }
  // 開閉の初期状態は属性に残すだけで本文を隠さず、操作の実現は利用側が決める。
  for (const [subsection, state] of collapsible) {
    subsection.properties = { ...subsection.properties, dataFold: state };
  }
  return result;
}

/**
 * 組み上がった要素が、書き手の書いた何にあたるか。名前はコンポーネントならその名前、
 * Markdownの記法ならその記法の名前とし、原稿を見ればidが分かるようにする。
 */
function kindOf(node: Element): string | null {
  const classes = classesOf(node);
  if (node.tagName === "figure" && classes.includes("media")) {
    if (classes.includes("svg-figure")) {
      return "svg";
    }
    return node.children.some((child) => isElement(child) && child.tagName === "video")
      ? "video"
      : "image";
  }
  if (node.tagName === "div") {
    if (classes.includes("callout")) return "callout";
    if (classes.includes("code-group")) return "code-group";
    if (classes.includes("code-block-wrapper")) {
      return "code-block";
    }
    if (classes.includes("table-wrap")) return "table";
  }
  if (node.tagName === "pre") return "code-block";
  if (node.tagName === "table") return "table";
  if (classes.includes("url-card")) return "url-card";
  if (classes.includes("tweet-card")) return "tweet-card";
  return null;
}

/** 中に別の要素を持てる種類。それ以外は1つの部品として扱い、中を数えない。 */
const CONTAINER_KINDS = new Set(["callout", "code-group"]);

function assignElementIds(
  children: (RootContent | ElementContent)[],
  counts: Map<string, number>,
): void {
  for (const child of children) {
    if (!isElement(child)) {
      continue;
    }
    const kind = kindOf(child);
    if (kind === null) {
      assignElementIds(child.children, counts);
      continue;
    }
    const number = (counts.get(kind) ?? 0) + 1;
    const limit = 10 ** ELEMENT_NUMBER_DIGITS - 1;
    if (number > limit) {
      throw new Error(
        `IdNumberOverflow: an article holds at most ${limit} ${kind} elements. Split the article.`,
      );
    }
    counts.set(kind, number);
    child.properties = {
      ...child.properties,
      id: `${kind}-${String(number).padStart(ELEMENT_NUMBER_DIGITS, "0")}`,
    };
    if (CONTAINER_KINDS.has(kind)) {
      assignElementIds(child.children, counts);
    }
  }
}

/**
 * 見出しのidを`<section>`へ移し、書き手が書いた要素へ種類ごとの連番idを振る。
 *
 * どの部品も組み上がった後に通すこと。見出しのidは`rehypeHeadingSlug`が振ったものを
 * そのまま移し、ここでは採番しない。
 */
export function rehypeArticleStructure() {
  return (tree: Root | Element): void => {
    tree.children = wrapSections(tree.children) as typeof tree.children;
    assignElementIds(tree.children, new Map());
  };
}
