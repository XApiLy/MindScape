import { strict as assert } from "node:assert";
import test from "node:test";
import { blocksToMarkdown, safeMarkdownUrl } from "./markdownContent.ts";

test("preserves text and reconstructs fenced code without executing it", () => {
  const markdown = blocksToMarkdown([
    { type: "text", text: "## 结论\n\n- 保留原文" },
    { type: "code", language: "ts", code: "const fence = ```;" },
    { type: "toolCallRef", toolRunId: "tool-1" },
  ]);

  assert.match(markdown, /^## 结论/);
  assert.match(markdown, /````ts\nconst fence = ```;\n````/);
  assert.match(markdown, /\[工具调用：tool-1\]/);
});

test("allows explicit web links and rejects executable or embedded protocols", () => {
  assert.equal(safeMarkdownUrl("https://example.com"), "https://example.com");
  assert.equal(safeMarkdownUrl("mailto:user@example.com"), "mailto:user@example.com");
  assert.equal(safeMarkdownUrl("#section"), "#section");
  assert.equal(safeMarkdownUrl("javascript:alert(1)"), "");
  assert.equal(safeMarkdownUrl("data:text/html,unsafe"), "");
  assert.equal(safeMarkdownUrl("file:///private/path"), "");
});
