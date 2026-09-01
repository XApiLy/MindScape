import assert from "node:assert/strict";
import test from "node:test";
import { chunkMockResponse } from "./mockStreamChunking.ts";

test("keeps Markdown line breaks and Unicode intact across mock stream chunks", () => {
  const markdown = "# 标题\n\n- [x] 完成\n\n~~~ts\nconst safe = true;\n~~~\n🧠";
  const chunks = chunkMockResponse(markdown);

  assert.equal(chunks.join(""), markdown);
  assert.equal(chunks.some((chunk) => chunk.includes("\n")), true);
  assert.equal(chunks.join("").includes("�"), false);
});
