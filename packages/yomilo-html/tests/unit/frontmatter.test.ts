import { expect, test } from "bun:test";
import { parseFrontmatter, validateFrontmatter } from "../../src/frontmatter";

function parse(source: string) {
  return validateFrontmatter(parseFrontmatter(source).frontmatter);
}

test("reads optional shared fields without imposing the site's vocabulary on a document", () => {
  expect(parse("# Title\n")).toBeNull();
  expect(parse("---\ncategory: 独自の分類\nreviewer: 田中\n---\n\n# Title\n")).toEqual({
    category: "独自の分類",
    tags: [],
    status: "published",
  });
});

test("rejects malformed values on specified fields", () => {
  expect(() => parse("---\ncategory: 12\n---\n")).toThrow(/category/);
  expect(() => parse("---\ntags: mcp\n---\n")).toThrow(/tags/);
  expect(() => parse("---\nstatus: wip\n---\n")).toThrow(/status/);
});
