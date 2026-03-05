/**
 * Markdown — RWE SSR markdown renderer component.
 *
 * Usage:
 *   import Markdown from "@/components/ui/markdown";
 *
 *   // Static content (children)
 *   <Markdown class="prose prose-invert">## Hello\n\n**bold** text</Markdown>
 *
 *   // Dynamic content (prop) — safe through base64 encoding
 *   <Markdown content={post.body} class="prose prose-invert max-w-none" />
 *
 * At SSR time the Rust processor finds the `data-rwe-md` attribute, base64-decodes
 * the markdown, renders it via pulldown-cmark, and replaces the placeholder with
 * a styled `<div class="prose ...">` containing the HTML output.
 *
 * For client-side hydrated markdown (reactive content), import from zeb/markdown instead:
 *   import { Markdown } from "zeb/markdown";
 */

interface MarkdownProps {
  content?: string;
  children?: string;
  class?: string;
}

export default function Markdown({ content, children, class: className }: MarkdownProps) {
  const text = content ?? (typeof children === "string" ? children : "") ?? "";

  // Encode to base64 so the Rust processor can decode the raw markdown safely —
  // JSX would otherwise HTML-escape the string, corrupting the markdown syntax.
  // btoa/encodeURIComponent handles the full Unicode range.
  const encoded =
    typeof btoa !== "undefined"
      ? btoa(unescape(encodeURIComponent(text)))
      : Buffer.from(text, "utf-8").toString("base64");

  return (
    <div
      class={`rwe-md-placeholder${className ? ` ${className}` : ""}`}
      data-rwe-md={encoded}
    />
  );
}
