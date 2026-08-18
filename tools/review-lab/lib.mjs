import { createHash, randomBytes } from "node:crypto";

export const VALID_BUILD_MODES = new Set(["preview", "full"]);
export const VALID_VERDICTS = new Set(["approved", "changes_requested", "rejected", "deferred"]);

export function cleanText(value, maxLength) {
  return String(value ?? "")
    .replace(/\u0000/g, "")
    .trim()
    .slice(0, maxLength);
}

export function slugify(value) {
  const slug = cleanText(value, 80)
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 36);
  return slug || "review";
}

export function createVersionId(title, now = new Date()) {
  const stamp = now.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  return `${stamp}-${slugify(title)}-${randomBytes(2).toString("hex")}`;
}

export function createAnchorId(now = new Date()) {
  const stamp = now.toISOString().replace(/\D/g, "").slice(2, 14);
  return `UI-${stamp}-${randomBytes(2).toString("hex").toUpperCase()}`;
}

export function validateVersionInput(input) {
  const title = cleanText(input?.title, 80);
  const author = cleanText(input?.author, 40);
  const summary = cleanText(input?.summary, 1200);
  const focus = cleanText(input?.focus, 600);
  const buildMode = cleanText(input?.buildMode, 20) || "preview";

  if (!title) throw new Error("请填写版本名称");
  if (!author) throw new Error("请填写提交员工");
  if (!summary) throw new Error("请说明本次推进内容");
  if (!VALID_BUILD_MODES.has(buildMode)) throw new Error("构建类型无效");

  return { title, author, summary, focus, buildMode };
}

export function validateReviewInput(input) {
  const author = cleanText(input?.author, 40);
  const verdict = cleanText(input?.verdict, 30);
  const note = cleanText(input?.note, 2000);

  if (!author) throw new Error("请填写评审人");
  if (!VALID_VERDICTS.has(verdict)) throw new Error("请选择有效的评审结论");
  if (!note) throw new Error("请填写具体反馈");

  return { author, verdict, note };
}

export function validateAnchorInput(input) {
  const author = cleanText(input?.author, 40);
  const title = cleanText(input?.title, 100);
  const note = cleanText(input?.note, 2000);
  const viewLabel = cleanText(input?.viewLabel, 100);
  const x = Number(input?.x);
  const y = Number(input?.y);
  const rawHint = input?.elementHint && typeof input.elementHint === "object" ? input.elementHint : {};
  const elementHint = {
    tag: cleanText(rawHint.tag, 40),
    role: cleanText(rawHint.role, 80),
    label: cleanText(rawHint.label, 160),
    text: cleanText(rawHint.text, 240),
    selector: cleanText(rawHint.selector, 300),
  };

  if (!author) throw new Error("请填写指向人");
  if (!title) throw new Error("请概括这个界面细节");
  if (!note) throw new Error("请说明希望讨论或确认什么");
  if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || x > 1 || y < 0 || y > 1) {
    throw new Error("视觉锚点坐标无效");
  }

  return { author, title, note, viewLabel, x, y, elementHint };
}

export function hashText(value) {
  return createHash("sha256").update(value).digest("hex");
}

export function trailerValue(body, name) {
  const escapedName = String(name).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return String(body ?? "").match(new RegExp(`^${escapedName}:\\s*(.+)$`, "im"))?.[1]?.trim() || "";
}

export function parseGitReviewSignal({ commit, gitAuthor, subject, body }) {
  const buildMode = trailerValue(body, "Review-Lab").toLowerCase();
  if (!VALID_BUILD_MODES.has(buildMode)) return null;
  return {
    input: {
      title: trailerValue(body, "Review-Title") || String(subject).replace(/^\[review(?::full)?\]\s*/i, "") || "Git 可评审版本",
      author: trailerValue(body, "Review-Author") || gitAuthor || "未知员工",
      summary: trailerValue(body, "Review-Summary") || subject,
      focus: trailerValue(body, "Review-Focus"),
      buildMode,
    },
    trigger: { type: "git-trailer", commit },
  };
}

export function publicVersion(version) {
  return {
    ...version,
    buildLogPath: undefined,
    artifactDirectory: undefined,
  };
}
