export type ImportIntakeFormat = "markdown" | "jsonl" | "text";

export type ImportIntakeIssue = {
  code: "emptySource" | "unsupportedFormat" | "sourceTooLarge";
  message: string;
};

export type ImportIntakeCandidate = {
  kind: "file" | "paste";
  displayName: string;
  format: ImportIntakeFormat | null;
  sizeBytes: number;
  issues: ImportIntakeIssue[];
};

export type ImportIntakeLimits = {
  maxBytes?: number;
};

const formatByExtension: Readonly<Record<string, ImportIntakeFormat>> = {
  md: "markdown",
  markdown: "markdown",
  jsonl: "jsonl",
  txt: "text",
};

function inspectSize(sizeBytes: number, limits: ImportIntakeLimits) {
  const issues: ImportIntakeIssue[] = [];
  if (sizeBytes === 0) {
    issues.push({ code: "emptySource", message: "来源内容为空，请选择包含会话内容的文件或文本。" });
  }
  if (limits.maxBytes !== undefined && sizeBytes > limits.maxBytes) {
    issues.push({
      code: "sourceTooLarge",
      message: `来源大小超过当前本地导入上限（${formatImportBytes(limits.maxBytes)}）。`,
    });
  }
  return issues;
}

export function inspectImportFile(
  file: Pick<File, "name" | "size">,
  limits: ImportIntakeLimits = {},
): ImportIntakeCandidate {
  const extension = file.name.split(".").pop()?.toLocaleLowerCase() ?? "";
  const format = formatByExtension[extension] ?? null;
  const issues = inspectSize(file.size, limits);

  if (!format) {
    issues.unshift({
      code: "unsupportedFormat",
      message: "M2 当前支持 Markdown、JSONL 和 TXT；其他格式不会被静默当作普通文本。",
    });
  }

  return {
    kind: "file",
    displayName: file.name,
    format,
    sizeBytes: file.size,
    issues,
  };
}

export function inspectPastedConversation(
  text: string,
  limits: ImportIntakeLimits = {},
): ImportIntakeCandidate {
  const sizeBytes = new TextEncoder().encode(text).byteLength;
  return {
    kind: "paste",
    displayName: "粘贴的会话文本",
    format: "text",
    sizeBytes,
    issues: inspectSize(text.trim() ? sizeBytes : 0, limits),
  };
}

export function formatImportBytes(sizeBytes: number) {
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function importFormatLabel(format: ImportIntakeFormat | null) {
  return format === "markdown"
    ? "Markdown"
    : format === "jsonl"
      ? "JSONL"
      : format === "text"
        ? "TXT / 纯文本"
        : "不支持";
}
