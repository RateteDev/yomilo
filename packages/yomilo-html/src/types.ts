import type { Frontmatter } from "./frontmatter.js";

export type { Frontmatter };

export type DiagnosticLevel = "error" | "warning" | "info";

export interface Diagnostic {
  level: DiagnosticLevel;
  message: string;
  node?: string;
}

export interface Heading {
  depth: number;
  text: string;
  slug: string;
}
