/**
 * zeb/markdown — Markdown component (SSR + client-aware).
 *
 * Usage:
 *   import { Markdown } from "zeb/markdown";
 *   <Markdown content={post.body} className="prose max-w-none" />
 *
 *   // Inline children
 *   <Markdown className="prose">## Heading\n\nSome **bold** text.</Markdown>
 *
 * `prose` styles the rendered headings, lists, links, code, quotes and tables
 * with the theme's colours in any project; `prose-sm` / `prose-lg` change the
 * size. `className` is the name every zeb/ui component takes; `class` works
 * too.
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
  className?: string;
  class?: string;
}

export default function Markdown({ content, children, className, class: htmlClass }: MarkdownProps) {
  const classes = [className, htmlClass].filter(Boolean).join(" ");
  const text = content ?? (typeof children === "string" ? children : "") ?? "";

  // Base64-encode markdown so JSX doesn't HTML-escape the content.
  const encoded =
    typeof btoa !== "undefined"
      ? btoa(unescape(encodeURIComponent(text)))
      : Buffer.from(text, "utf-8").toString("base64");

  return (
    <div
      class={`rwe-md-placeholder${classes ? ` ${classes}` : ""}`}
      data-rwe-md={encoded}
      data-zeb-lib="markdown"
    />
  );
}
