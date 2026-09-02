import assert from "node:assert/strict";
import test from "node:test";
import {
  formatImportBytes,
  importFormatLabel,
  inspectImportFile,
  inspectPastedConversation,
} from "./importIntake.ts";

test("recognizes the M2 generic import formats without parsing file contents", () => {
  assert.deepEqual(inspectImportFile({ name: "conversation.MARKDOWN", size: 2048 }), {
    kind: "file",
    displayName: "conversation.MARKDOWN",
    format: "markdown",
    sizeBytes: 2048,
    issues: [],
  });
  assert.equal(inspectImportFile({ name: "turns.jsonl", size: 12 }).format, "jsonl");
  assert.equal(inspectImportFile({ name: "notes.txt", size: 12 }).format, "text");
});

test("does not silently downgrade unsupported or empty files", () => {
  const candidate = inspectImportFile({ name: "archive.json", size: 0 });
  assert.equal(candidate.format, null);
  assert.deepEqual(candidate.issues.map((issue) => issue.code), [
    "unsupportedFormat",
    "emptySource",
  ]);
});

test("applies a caller-owned size policy instead of inventing a UI limit", () => {
  const candidate = inspectImportFile(
    { name: "large.md", size: 2049 },
    { maxBytes: 2048 },
  );
  assert.deepEqual(candidate.issues.map((issue) => issue.code), ["sourceTooLarge"]);
});

test("counts pasted text as UTF-8 and rejects whitespace-only input", () => {
  assert.equal(inspectPastedConversation("你好").sizeBytes, 6);
  assert.equal(inspectPastedConversation("  \n ").issues[0]?.code, "emptySource");
});

test("formats intake metadata for the preview bill", () => {
  assert.equal(formatImportBytes(800), "800 B");
  assert.equal(formatImportBytes(1536), "1.5 KB");
  assert.equal(importFormatLabel("jsonl"), "JSONL");
  assert.equal(importFormatLabel(null), "不支持");
});
