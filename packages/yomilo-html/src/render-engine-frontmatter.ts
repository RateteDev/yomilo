import type { RenderSegment } from "@yomilo/wasm";
import { parse as parseYaml } from "yaml";

export function readEngineFrontmatter(segments: RenderSegment[]): unknown {
  const frontmatter = segments.find((segment) => segment.content.kind === "frontmatter")?.content;
  if (frontmatter?.kind !== "frontmatter") return null;
  if (frontmatter.value.format !== "yaml") {
    throw new Error(`Unsupported frontmatter format: ${frontmatter.value.format}`);
  }
  return parseYaml(frontmatter.value.value);
}
