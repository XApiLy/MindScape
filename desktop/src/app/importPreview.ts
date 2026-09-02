import type { ImportedMessage, RawImportContentProjection } from "../domain";
import { blocksToMarkdown } from "./markdownContent.ts";

export const IMPORT_PREVIEW_PAGE_SIZE = 8;

export function importedRoleLabel(role: ImportedMessage["role"]) {
  if (role === "user") return "用户";
  if (role === "assistant") return "助手";
  if (role === "system") return "系统";
  return "导入内容";
}

export function importMessageMarkdown(message: ImportedMessage) {
  return blocksToMarkdown(message.contentBlocks);
}

export function nextImportPreviewCount(
  current: number,
  total: number,
  pageSize = IMPORT_PREVIEW_PAGE_SIZE,
) {
  return Math.min(Math.max(0, total), Math.max(0, current) + Math.max(1, pageSize));
}

export function rawImportContentIntegrityIssue(
  projection: RawImportContentProjection,
  expectedSourceId: string,
) {
  if (projection.sourceId !== expectedSourceId) {
    return "受控原文返回了不匹配的来源标识。";
  }
  if (!/^[a-f0-9]{64}$/i.test(projection.contentHash)) {
    return "受控原文缺少有效的内容指纹。";
  }
  const visibleBytes = new TextEncoder().encode(projection.content).byteLength;
  if (projection.byteLength < visibleBytes) {
    return "受控原文预览超过了声明的完整字节数。";
  }
  if (!projection.truncated && projection.byteLength !== visibleBytes) {
    return "完整受控原文的字节数与预览内容不一致。";
  }
  return null;
}
