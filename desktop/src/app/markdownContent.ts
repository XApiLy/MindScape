import type { ContentBlock } from "../domain";

function codeFenceFor(code: string) {
  const longestRun = Math.max(0, ...Array.from(code.matchAll(/`+/g), (match) => match[0].length));
  return "`".repeat(Math.max(3, longestRun + 1));
}

function safeLanguageLabel(language: string | null) {
  return language?.trim().match(/^[a-z0-9_+#.-]+$/i)?.[0] ?? "";
}

/** Reconstructs renderable Markdown without changing the stored content blocks. */
export function blocksToMarkdown(blocks: readonly ContentBlock[]) {
  return blocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "code") {
        const fence = codeFenceFor(block.code);
        return `${fence}${safeLanguageLabel(block.language)}\n${block.code}\n${fence}`;
      }
      if (block.type === "link") {
        return block.label ? `[${block.label.replaceAll("]", "\\]")}](${block.url})` : block.url;
      }
      if (block.type === "attachmentRef") return `[附件：${block.displayName}]`;
      if (block.type === "toolCallRef") return `[工具调用：${block.toolRunId}]`;
      if (block.type === "toolResultRef") return `[工具结果：${block.toolRunId}]`;
      return `[暂不支持的内容：${block.originalType}]`;
    })
    .join("\n\n");
}

/** Allows only explicitly safe navigation schemes in rendered Markdown. */
export function safeMarkdownUrl(url: string) {
  const trimmed = url.trim();
  if (/^(https?:|mailto:)/i.test(trimmed) || trimmed.startsWith("#")) return trimmed;
  return "";
}
