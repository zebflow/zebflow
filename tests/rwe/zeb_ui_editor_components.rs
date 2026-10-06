//! Editor extensions drawn by Zeb React components — rendered by the real
//! compiler and SSR engine. The page's server HTML (DocumentView), the HTML
//! string (renderDocumentHtml) and the props a component gets are checked
//! here; the editor's node view is checked in the browser (e2e).

use super::zeb_ui_editor::{between, render, unescape};

/// A project's extension file: one written in JSX with a component for the
/// page and one for the editor, configured with options; a component that
/// tries to publish what a document may not; and a story block with
/// default libraries.
const COMPONENTS: &str = r#"import { useState } from "zeb/react";
import { defineExtension } from "zeb/ui/editor-extension";
import { potoruExtension } from "zeb/ui/editor-potoru";

function StatusBadge({ node, attrs, options, edit, selected, readOnly, update }) {
  const [seen] = useState(options.labels[attrs.state] || attrs.state);
  return <span data-status={attrs.state} data-type={node.type} data-props={`${edit}/${selected}/${readOnly}/${typeof update}`} className="rounded-full px-2 text-xs">{seen}</span>;
}

function StatusEditor({ attrs, update }) {
  return <select value={attrs.state} onChange={(e) => update({ state: e.target.value })}><option value="draft">Draft</option></select>;
}

export const status = defineExtension({
  name: "status",
  node: { group: "inline", inline: true, atom: true, attrs: { state: { default: "draft" } } },
  options: { labels: { draft: "Draft", review: "In review" } },
  component: StatusBadge,
  editComponent: StatusEditor,
  text: (node) => node.attrs.state || "draft",
});

function Card({ attrs }) {
  return <div className="card" style={{ color: "red" }} onmouseover="steal()">
    <a href={attrs.href} target="_top">{attrs.label}</a>
    <script>{"alert(1)"}</script>
    <iframe src={attrs.href} />
    <img src={attrs.href} srcset="x.png 2x" alt="" />
    <p dangerouslySetInnerHTML={{ __html: "<b>raw</b>" }} />
  </div>;
}

export const card = defineExtension({
  name: "card",
  node: { group: "block", atom: true, attrs: { href: { default: "" }, label: { default: "" } } },
  component: Card,
});

export const COMPONENTS = [
  status,
  card,
  potoruExtension({ libraries: ["https://example.com/libs/basic.potolib", "/_files/libs/shapes.potolib"] }),
];
"#;

fn page(doc: &str, body: &str) -> String {
    format!(
        r#"import {{ DocumentView, renderDocumentHtml, documentText }} from "zeb/ui/editor-render";
import {{ COMPONENTS }} from "@/shared/editor/extensions/components";
const DOC = {doc};
export default function Page() {{
  return <main>{body}<section id="elements"><DocumentView doc={{DOC}} extensions={{COMPONENTS}} /></section><pre id="string">{{renderDocumentHtml(DOC, {{ extensions: COMPONENTS }})}}</pre><p id="text">{{documentText(DOC, {{ extensions: COMPONENTS }})}}</p></main>;
}}
"#
    )
}

fn renders(doc: &str, body: &str) -> (String, String) {
    let html = render(&page(doc, body), &[("shared/editor/extensions/components.tsx", COMPONENTS)]);
    let elements = between(&html, "<section id=\"elements\"><div data-slot=\"document\">", "</div></section>").to_string();
    assert_eq!(elements, unescape(between(&html, "<pre id=\"string\">", "</pre>")), "DocumentView and renderDocumentHtml differ");
    (elements, html)
}

/// The page's server HTML and the HTML string are the same bytes; the
/// component got the documented props (on the page: not editing, not
/// selected, read-only, a do-nothing update) and its hooks ran on the server.
#[test]
fn a_component_node_renders_the_same_on_the_page_and_in_the_string() {
    let (elements, html) = renders(
        r#"{ type: "doc", content: [{ type: "paragraph", content: [
          { type: "text", text: "Survey " },
          { type: "status", attrs: { state: "review" } },
          { type: "text", text: " then " },
          { type: "status", attrs: {} }
        ] }] }"#,
        "",
    );
    assert!(elements.contains("<p class=\"my-1.5\">Survey <span data-status=\"review\" data-type=\"status\" data-props=\"false/false/true/function\" class=\"rounded-full px-2 text-xs\">In review</span> then <span data-status=\"draft\""), "{elements}");
    assert!(!elements.contains("<select"), "the editor's component never reaches the page:\n{elements}");
    assert!(html.contains("<p id=\"text\">Survey review then draft</p>"), "{html}");
}

/// A component's page output passes the document allowlist, whatever the
/// stored attrs say: no script, iframe, style, string handler, srcset,
/// raw HTML or `javascript:` link.
#[test]
fn a_components_output_is_sanitized() {
    let (elements, _) = renders(
        r#"{ type: "doc", content: [{ type: "card", attrs: { href: "javascript:alert(1)", label: "<b>open</b>" } }] }"#,
        "",
    );
    assert_eq!(elements, "<div class=\"card\"><a href=\"#\">&lt;b&gt;open&lt;/b&gt;</a><img src=\"#\" alt=\"\"><p></p></div>");
}

/// The story block's player gets the extension's default libraries and the
/// block's own, merged — the block's entry replaces a default of the same
/// file name, a bad stored address is left out — in exactly the
/// `data-config` PotoPlayer prints for that list.
#[test]
fn a_potoru_block_plays_with_default_and_own_libraries() {
    let doc = r#"{ type: "doc", content: [{ type: "potoru", attrs: { key: "intro", src: "/_files/stories/intro.poto",
      libraries: ["/_files/libs/basic.potolib", "https://example.com/libs/extra.potolib", "http://example.com/libs/bad.potolib", "javascript:alert(1)"] } }] }"#;
    let player = r#"<section id="player"><PotoPlayer src="/_files/stories/intro.poto" libraries={["/_files/libs/shapes.potolib", "/_files/libs/basic.potolib", "https://example.com/libs/extra.potolib"]} controls /></section>"#;
    let (elements, html) = renders(doc, player);
    let config = between(&elements, "data-config=\"", "\"");
    assert_eq!(config, between(between(&html, "<section id=\"player\">", "</section>"), "data-config=\"", "\""), "\nblock: {elements}");
    assert_eq!(
        unescape(config),
        r#"{"src":"/_files/stories/intro.poto","libraries":["/_files/libs/shapes.potolib","/_files/libs/basic.potolib","https://example.com/libs/extra.potolib"],"controls":true}"#
    );
    assert!(!elements.contains("bad.potolib") && !elements.contains("javascript:"), "{elements}");
}

/// The contract is checked when the list is built: a default library that
/// is not https or a site path, a component node with content, a node with
/// both render and component.
#[test]
fn component_and_library_mistakes_are_refused() {
    let probe = r#"import { defineExtension } from "zeb/ui/editor-extension";
import { potoruExtension } from "zeb/ui/editor-potoru";
function attempt(make) { try { make(); return "ok"; } catch (err) { return err.message; } }
const C = () => null;
export default function Page() {
  return <main>
    <p id="http">{attempt(() => potoruExtension({ libraries: ["http://example.com/a.potolib"] }))}</p>
    <p id="relative">{attempt(() => potoruExtension({ libraries: ["libs/a.potolib"] }))}</p>
    <p id="content">{attempt(() => defineExtension({ name: "box", node: { group: "block", content: "inline*" }, component: C }))}</p>
    <p id="both">{attempt(() => defineExtension({ name: "box", node: { group: "block", atom: true }, component: C, render: () => null }))}</p>
  </main>;
}
"#;
    let html = render(probe, &[]);
    assert!(html.contains("<p id=\"http\">potoruExtension &quot;potoru&quot;: library &quot;http://example.com/a.potolib&quot; must be an https:// address or a path on this site (/…)</p>"), "{html}");
    assert!(html.contains("<p id=\"relative\">potoruExtension &quot;potoru&quot;: library &quot;libs/a.potolib&quot; must be"), "{html}");
    assert!(html.contains("<p id=\"content\">editor extension &quot;box&quot;: &quot;box&quot; is drawn by a component, so it is an atom: give it attrs, not content</p>"), "{html}");
    assert!(html.contains("<p id=\"both\">editor extension &quot;box&quot;: &quot;box&quot; has both render and component; the page draws it with one of them</p>"), "{html}");
}
