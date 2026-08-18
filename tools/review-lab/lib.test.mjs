import assert from "node:assert/strict";
import test from "node:test";
import {
  createVersionId,
  createAnchorId,
  parseGitReviewSignal,
  slugify,
  validateReviewInput,
  validateAnchorInput,
  validateVersionInput,
} from "./lib.mjs";

test("slugify keeps readable Chinese version names", () => {
  assert.equal(slugify("  Chat 流式状态 V2  "), "chat-流式状态-v2");
});

test("version input is normalized and defaults to preview", () => {
  assert.deepEqual(
    validateVersionInput({ title: " Alpha ", author: " 员工06 ", summary: " 首轮视觉基线 " }),
    {
      title: "Alpha",
      author: "员工06",
      summary: "首轮视觉基线",
      focus: "",
      buildMode: "preview",
    },
  );
});

test("review input rejects unknown verdicts", () => {
  assert.throws(
    () => validateReviewInput({ author: "负责人", verdict: "maybe", note: "看看" }),
    /评审结论/,
  );
});

test("version id contains a stable timestamp and readable slug", () => {
  const id = createVersionId("节点动效", new Date("2026-08-18T09:30:00.000Z"));
  assert.match(id, /^20260818T093000Z-节点动效-[a-f0-9]{4}$/);
});

test("Git trailers form one explicit automated review signal", () => {
  assert.deepEqual(
    parseGitReviewSignal({
      commit: "abc123",
      gitAuthor: "Developer",
      subject: "[review] 完成停止交互",
      body: "[review] 完成停止交互\n\nReview-Lab: preview\nReview-Author: 员工04\nReview-Focus: 停止中反馈",
    }),
    {
      input: {
        title: "完成停止交互",
        author: "员工04",
        summary: "[review] 完成停止交互",
        focus: "停止中反馈",
        buildMode: "preview",
      },
      trigger: { type: "git-trailer", commit: "abc123" },
    },
  );
});

test("ordinary commits never trigger a build", () => {
  assert.equal(
    parseGitReviewSignal({ commit: "abc123", gitAuthor: "Developer", subject: "fix styles", body: "fix styles" }),
    null,
  );
});

test("visual anchor keeps normalized coordinates and safe element context", () => {
  assert.deepEqual(
    validateAnchorInput({
      author: " 项目负责人 ",
      title: " 停止按钮 ",
      note: " 反馈不够明显 ",
      viewLabel: "流式中",
      x: 0.42,
      y: 0.81,
      elementHint: { tag: "BUTTON", label: "停止生成", text: "停止", selector: "button.stop" },
    }),
    {
      author: "项目负责人",
      title: "停止按钮",
      note: "反馈不够明显",
      viewLabel: "流式中",
      x: 0.42,
      y: 0.81,
      elementHint: { tag: "BUTTON", role: "", label: "停止生成", text: "停止", selector: "button.stop" },
    },
  );
});

test("anchor id is short enough to paste into a discussion", () => {
  assert.match(createAnchorId(new Date("2026-08-18T09:45:12.000Z")), /^UI-260818094512-[A-F0-9]{4}$/);
});
