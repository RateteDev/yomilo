import type { Element, Node as HastNode, Text } from "hast";

const SECTION_DEPTH = 2;
const SUBSECTION_DEPTH = 3;

/**
 * 見出しの番号の桁数。画面に出すセクションの番号と同じ2桁に固定し、どの文書でも
 * 番号からidを決め打ちで作れるようにする。収まらない文書は検査が落とす。
 */
export const HEADING_NUMBER_DIGITS = 2;

export type HeadingIdGenerator = (depth: number) => string | undefined;

function headingNumber(value: number): string {
  const limit = 10 ** HEADING_NUMBER_DIGITS - 1;
  if (value > limit) {
    throw new Error(
      `IdNumberOverflow: heading ids hold at most ${limit} entries per level. Split the article or section.`,
    );
  }
  return String(value).padStart(HEADING_NUMBER_DIGITS, "0");
}

/**
 * webとCLIで同じ見出しIDを発行する。セクションは文書順に`section-01`、サブセクションは
 * セクションの中で数えて`section-01-01`とし、どちらも見出しの文字に依存させない。
 * セクションより前に置かれたサブセクションは数える先が無いため、idを持たない。
 */
export function createHeadingIdGenerator(): HeadingIdGenerator {
  let sectionNumber = 0;
  let subsectionNumber = 0;

  return (depth) => {
    if (depth === SECTION_DEPTH) {
      sectionNumber += 1;
      subsectionNumber = 0;
      return `section-${headingNumber(sectionNumber)}`;
    }
    if (depth === SUBSECTION_DEPTH && sectionNumber > 0) {
      subsectionNumber += 1;
      return `section-${headingNumber(sectionNumber)}-${headingNumber(subsectionNumber)}`;
    }
    return undefined;
  };
}

/** 見出し配下の全テキストを、描画経路に依存しない形で連結する。 */
export function extractHeadingText(node: HastNode): string {
  if (node.type === "text") {
    return (node as Text).value;
  }
  if ("children" in node && Array.isArray((node as Element).children)) {
    return (node as Element).children.map(extractHeadingText).join("");
  }
  return "";
}
