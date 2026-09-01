import {
  Children,
  isValidElement,
  memo,
  useDeferredValue,
  useState,
  type ComponentPropsWithoutRef,
  type Ref,
  type ReactNode,
} from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Check, Copy, ImageOff } from "lucide-react";
import { safeMarkdownUrl } from "../app/markdownContent";
import "./safeMarkdown.css";

const REMARK_PLUGINS = [remarkGfm];

function nodeText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeText).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) return nodeText(node.props.children);
  return "";
}

function MarkdownPre({ children }: ComponentPropsWithoutRef<"pre">) {
  const [copied, setCopied] = useState(false);
  const child = Children.toArray(children)[0];
  const className = isValidElement<{ className?: string }>(child) ? child.props.className : undefined;
  const language = className?.match(/language-([^\s]+)/)?.[1] ?? "text";
  const code = nodeText(child).replace(/\n$/, "");

  const copyCode = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <div className="markdown-code-block">
      <div className="markdown-code-toolbar" data-markdown-copy-exclude>
        <span>{language}</span>
        <button type="button" onClick={() => void copyCode()} aria-label="复制代码">
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? "已复制" : "复制代码"}
        </button>
      </div>
      <pre>{children}</pre>
    </div>
  );
}

const MARKDOWN_COMPONENTS: Components = {
  pre: MarkdownPre,
  a: ({ href, children, ...props }) => href ? (
    <a {...props} href={href} target="_blank" rel="noreferrer noopener">{children}</a>
  ) : <span className="markdown-disabled-link">{children}</span>,
  img: ({ alt }) => (
    <span className="markdown-image-placeholder" role="note">
      <ImageOff aria-hidden="true" />图片未加载{alt ? `：${alt}` : ""}
    </span>
  ),
  table: ({ children, ...props }) => (
    <div className="markdown-table-scroll" tabIndex={0} aria-label="可横向滚动的表格">
      <table {...props}>{children}</table>
    </div>
  ),
};

type SafeMarkdownProps = {
  markdown: string;
  streaming?: boolean;
  className?: string;
  contentRef?: Ref<HTMLDivElement>;
};

export function renderedMarkdownText(surface: HTMLElement | null) {
  if (!surface) return "";
  const excluded = Array.from(surface.querySelectorAll<HTMLElement>("[data-markdown-copy-exclude]"));
  const hiddenStates = excluded.map((element) => element.hidden);
  excluded.forEach((element) => { element.hidden = true; });
  try {
    return surface.innerText.trim();
  } finally {
    excluded.forEach((element, index) => { element.hidden = hiddenStates[index]; });
  }
}

export const SafeMarkdown = memo(function SafeMarkdown({
  markdown,
  streaming = false,
  className,
  contentRef,
}: SafeMarkdownProps) {
  const deferredMarkdown = useDeferredValue(markdown);
  const visibleMarkdown = streaming ? deferredMarkdown : markdown;

  return (
    <div ref={contentRef} className={`markdown-reading-surface${streaming ? " is-streaming" : ""}${className ? ` ${className}` : ""}`}>
      <ReactMarkdown
        remarkPlugins={REMARK_PLUGINS}
        skipHtml
        urlTransform={safeMarkdownUrl}
        components={MARKDOWN_COMPONENTS}
      >
        {visibleMarkdown}
      </ReactMarkdown>
    </div>
  );
});
