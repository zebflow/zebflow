/**
 * zeb/markdown — Markdown component (SSR + client-aware).
 *
 * Usage:
 *   import Markdown from "zeb/markdown";
 *   <Markdown content={post.body} class="prose prose-invert max-w-none" />
 *
 *   // Inline children
 *   <Markdown class="prose">## Heading\n\nSome **bold** text.</Markdown>
 *
 * For direct client-side rendering (e.g., reactive preview), use renderMarkdown:
 *   import { renderMarkdown } from "zeb/markdown";
 *   element.innerHTML = renderMarkdown(markdownText);
 *
 * SSR: the Rust post-processor finds `data-rwe-md` and replaces with rendered HTML.
 * Client: `data-zeb-lib="markdown"` marks the element for client bundle awareness.
 */

export const app = {};

interface MarkdownProps {
  content?: string;
  children?: string;
  class?: string;
}

export default function Markdown({ content, children, class: className }: MarkdownProps) {
  const text = content ?? (typeof children === "string" ? children : "") ?? "";

  // Base64-encode markdown so JSX doesn't HTML-escape the content.
  const encoded =
    typeof btoa !== "undefined"
      ? btoa(unescape(encodeURIComponent(text)))
      : Buffer.from(text, "utf-8").toString("base64");

  return (
    <div
      class={`rwe-md-placeholder${className ? ` ${className}` : ""}`}
      data-rwe-md={encoded}
      data-zeb-lib="markdown"
    />
  );
}
