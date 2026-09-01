import assert from "node:assert/strict";
import test from "node:test";
import {
  importedRoleLabel,
  importMessageMarkdown,
  nextImportPreviewCount,
  rawImportContentIntegrityIssue,
} from "./importPreview.ts";
import type { ImportedMessage } from "../domain/index.ts";

test("keeps imported Markdown content separate from its safe rendered projection", () => {
  const message: ImportedMessage = {
    id: "message-1",
    importRevisionId: "revision-1",
    role: "assistant",
    contentBlocks: [
      { type: "text", text: "# 原始标题\n\n- [x] 保留标记" },
      { type: "code", language: "ts", code: "const value = 1;" },
    ],
    occurredAt: null,
    sourceLocator: "line:3",
    parentImportedMessageId: null,
    platformExtension: null,
  };

  assert.equal(importedRoleLabel(message.role), "助手");
  assert.equal(
    importMessageMarkdown(message),
    "# 原始标题\n\n- [x] 保留标记\n\n```ts\nconst value = 1;\n```",
  );
  assert.equal(message.contentBlocks[0]?.type, "text");
});

test("reveals imported messages in bounded pages without exceeding the bundle", () => {
  assert.equal(nextImportPreviewCount(8, 27), 16);
  assert.equal(nextImportPreviewCount(24, 27), 27);
  assert.equal(nextImportPreviewCount(0, 3), 3);
  assert.equal(nextImportPreviewCount(8, 27, 0), 9);
});

test("accepts only bounded raw content for the requested registered source", () => {
  const projection = {
    sourceId: "source-1",
    contentHash: "a".repeat(64),
    byteLength: 12,
    content: "你好",
    truncated: true,
  };

  assert.equal(rawImportContentIntegrityIssue(projection, "source-1"), null);
  assert.match(
    rawImportContentIntegrityIssue({ ...projection, sourceId: "source-2" }, "source-1") ?? "",
    /来源标识/,
  );
  assert.match(
    rawImportContentIntegrityIssue({ ...projection, truncated: false }, "source-1") ?? "",
    /字节数/,
  );
  assert.match(
    rawImportContentIntegrityIssue({ ...projection, contentHash: "not-a-hash" }, "source-1") ?? "",
    /内容指纹/,
  );
});
