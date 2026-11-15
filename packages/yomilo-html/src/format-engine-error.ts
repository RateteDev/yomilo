import type { JsonApiError, RenderSegment, Segment } from "@yomilo/wasm";

type Position = { line: number; column: number };
type LocatedDiagnostic = JsonApiError["diagnostics"][number];

function at(position: Position | null): string {
  return position ? `${position.line}:${position.column}` : "?:?";
}

function indent(message: string): string {
  return message.replace(/\r?\n/g, "\n    ");
}

function diagnosticLines(diagnostics: LocatedDiagnostic[]): string[] {
  return diagnostics.flatMap((diagnostic) => [
    `  ${at(diagnostic.span?.start ?? null)}  ${diagnostic.class} ${diagnostic.code}: ${indent(diagnostic.message)}`,
    ...diagnostic.related.map(
      (related) => `    ${at(related.span.start)}  related: ${indent(related.message)}`,
    ),
  ]);
}

/** エンジン契約の診断だけを整形する。既存ValidationIssueへの変換や停止・警告の判断はしない。 */
export function formatEngineError(filePath: string, error: JsonApiError): string {
  const lines = [filePath, `  ${error.kind}: ${indent(error.message)}`];
  if (error.diagnostics.length > 0) {
    lines.push(...diagnosticLines(error.diagnostics));
  } else {
    const preparation = error.preparation;
    const position =
      preparation && "value" in preparation && "span" in preparation.value
        ? preparation.value.span.start
        : error.inputLocation;
    lines.push(`  ${at(position)}  ${preparation?.kind ?? error.kind}: ${indent(error.message)}`);
  }
  return lines.join("\n");
}

/** 規約に反したセグメントの診断を`formatEngineError`と同じ形で整形する。反したセグメントが無ければnull。 */
export function formatRejectedSegments(
  filePath: string,
  segments: Array<Segment | RenderSegment>,
): string | null {
  const diagnostics = segments.flatMap((segment) =>
    segment.content.kind === "rejected" ? segment.content.value.diagnostics : [],
  );
  if (diagnostics.length === 0) return null;
  const rejected = segments.filter((segment) => segment.content.kind === "rejected").length;
  return [
    filePath,
    `  structure: ${rejected} segment(s) violate the strict Markdown rules`,
    ...diagnosticLines(diagnostics),
  ].join("\n");
}
