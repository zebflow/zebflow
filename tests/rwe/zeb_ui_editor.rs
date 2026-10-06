//! The composable editor's server half: a document with extension nodes,
//! rendered by the real compiler and the real SSR engine — no browser.
//!
//! The project shape is the documented one: extensions live in
//! `shared/editor/extensions/`, and the page passes the same list to
//! `<DocumentView>` and to `renderDocumentHtml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{
    ReactiveWebEngine, ReactiveWebOptions, RenderContext, RweReactiveWebEngine, TemplateOptions,
    TemplateSource,
};

fn library_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("blessed/source-libraries/ui/0.1/src")
}

/// A project extension file: the shipped extensions configured, plus one of
/// the project's own written with `defineExtension`.
const PROJECT_EXTENSIONS: &str = r#"import { defineExtension } from "zeb/ui/editor-extension";
import { calloutExtension } from "zeb/ui/editor-callout";
import { tableExtension } from "zeb/ui/editor-table";
import { figureExtension } from "zeb/ui/editor-figure";
import { embedExtension } from "zeb/ui/editor-embed";
import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";

export const recipe = defineExtension({
  name: "recipe",
  node: { group: "block", atom: true, attrs: { key: { default: "" }, servings: { default: 1 } } },
  render: (node, r) => r.h("div", { class: "rounded-md border border-border p-2", "data-recipe": node.attrs.key }, `Serves ${node.attrs.servings}`),
  text: (node) => `recipe ${node.attrs.key}`,
});

export const EXTENSIONS = [
  calloutExtension(),
  tableExtension(),
  figureExtension(),
  embedExtension({ name: "product", label: "Product", route: "/api/products/search" }),
  mentionExtension({ name: "person", route: "/api/people/search" }),
  citationExtension({ route: "/api/works/search" }),
  recipe,
];
"#;

/// Render `page` (a page module body) inside a project whose
/// `shared/editor/extensions/index.tsx` is [`PROJECT_EXTENSIONS`].
pub(super) fn render(page: &str, extra: &[(&str, &str)]) -> String {
    render_with_script(page, extra).0
}

/// [`render`], plus the page's client script — what the browser runs.
pub(super) fn render_with_script(page: &str, extra: &[(&str, &str)]) -> (String, String) {
    let tmp = tempfile::Builder::new().prefix("zeb-ui-editor-").tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("shared/editor/extensions")).unwrap();
    std::fs::write(root.join("shared/editor/extensions/index.tsx"), PROJECT_EXTENSIONS).unwrap();
    for (rel, source) in extra {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let page_path = root.join("page.tsx");
    std::fs::write(&page_path, page).unwrap();
    let mut library_roots = BTreeMap::new();
    library_roots.insert("zeb/ui".to_string(), library_src());
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let compiled = engine
        .compile_template(
            &TemplateSource { id: "editor.page".to_string(), source_path: Some(page_path), markup: page.to_string() },
            &language,
            &ReactiveWebOptions {
                templates: TemplateOptions { library_roots, template_root: Some(root.clone()), style_entries: Vec::new() },
                ..Default::default()
            },
        )
        .expect("page compiles");
    let rendered = engine
        .render(
            &compiled,
            json!({}),
            &language,
            &RenderContext { route: "/doc".to_string(), request_id: "req-doc".to_string(), metadata: json!({}), enabled_libraries: Vec::new() },
        )
        .expect("page renders");
    assert!(!rendered.html.contains("RWE component error"), "a component threw:\n{}", rendered.html);
    let script = rendered.compiled_scripts.iter().map(|s| s.content.as_str()).collect::<Vec<_>>().join("\n");
    (rendered.html, script)
}

/// Every base block and every shipped extension, plus a project extension,
/// a citation cited twice, and a node nothing knows.
const DOC: &str = r#"{
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Field notes" }] },
    { type: "paragraph", content: [
      { type: "text", text: "Met " },
      { type: "person", attrs: { id: "p1", label: "Alex Example", href: "/people/p1", snapshot: { description: "Maps" } } },
      { type: "text", text: " who wrote " },
      { type: "citation", attrs: { id: "10.5555/a", label: "Example 2020", href: null, snapshot: { authors: ["Example, A.", "Sample, S."], year: 2020, title: "On samples", container: "Journal of Examples", doi: "10.5555/a" } } },
      { type: "text", text: " and " },
      { type: "citation", attrs: { id: "10.5555/b", label: "Sample 2021", href: null, snapshot: { authors: "Sample, S.", year: 2021, title: "More samples", url: "https://example.com/more" } } },
      { type: "text", text: ", again ", marks: [{ type: "bold" }] },
      { type: "citation", attrs: { id: "10.5555/a", label: "Example 2020", href: null, snapshot: { title: "On samples" } } },
      { type: "text", text: " <tags> & \"quotes\"", marks: [{ type: "link", attrs: { href: "https://example.com/x?a=1&b=2", title: null } }, { type: "italic" }] }
    ] },
    { type: "callout", attrs: { icon: "!", tone: "warning" }, content: [{ type: "paragraph", content: [{ type: "text", text: "Mind the step." }] }] },
    { type: "table", content: [
      { type: "table_row", content: [{ type: "table_header", content: [{ type: "text", text: "Site" }] }, { type: "table_header", content: [{ type: "text", text: "Count" }] }] },
      { type: "table_row", content: [{ type: "table_cell", content: [{ type: "text", text: "site-a" }] }, { type: "table_cell", content: [{ type: "text", text: "12" }] }] }
    ] },
    { type: "figure", attrs: { src: "/files/demo.png", alt: "A demo", ref: "uploads/demo.png", credit: "Photo: Example" }, content: [{ type: "text", text: "The demo site" }] },
    { type: "product", attrs: { key: "notebook", snapshot: { title: "Field notebook", description: "Dot grid", href: "/shop/notebook", image: "/files/notebook.png", meta: "In stock" } } },
    { type: "recipe", attrs: { key: "soup", servings: 4 } },
    { type: "todo_list", content: [{ type: "todo_item", attrs: { checked: true }, content: [{ type: "paragraph", content: [{ type: "text", text: "Done" }] }] }] },
    { type: "code_block", attrs: { language: "js" }, content: [{ type: "text", text: "const a = 1 < 2;" }] },
    { type: "pullquote", attrs: { by: "someone" }, content: [{ type: "paragraph", content: [{ type: "text", text: "Kept, not dropped." }] }] }
  ]
}"#;

pub(super) fn between<'a>(html: &'a str, start: &str, end: &str) -> &'a str {
    let from = html.find(start).unwrap_or_else(|| panic!("{start} not in:\n{html}")) + start.len();
    let to = html[from..].find(end).unwrap_or_else(|| panic!("{end} not after {start} in:\n{html}")) + from;
    &html[from..to]
}

pub(super) fn unescape(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

/// The page renders the document twice: as elements (`<DocumentView>`) and as
/// the `renderDocumentHtml` string, printed as text. Returns both.
fn both_renders(doc: &str) -> (String, String) {
    let page = format!(
        r#"import {{ DocumentView, renderDocumentHtml }} from "zeb/ui/editor-render";
import {{ EXTENSIONS }} from "@/shared/editor/extensions/index";
const DOC = {doc};
export default function Page() {{
  return <main><section id="elements"><DocumentView doc={{DOC}} extensions={{EXTENSIONS}} /></section><pre id="string">{{renderDocumentHtml(DOC, {{ extensions: EXTENSIONS }})}}</pre></main>;
}}
"#
    );
    let html = render(&page, &[]);
    let elements = between(&html, "<section id=\"elements\"><div data-slot=\"document\">", "</div></section>").to_string();
    let string = unescape(between(&html, "<pre id=\"string\">", "</pre>"));
    (elements, string)
}

/// Server elements and the HTML string are the same bytes: one walk, one
/// allowlist, one escaping. (The browser runs the same walk; the e2e spec
/// compares its output against this server's.)
#[test]
fn elements_and_the_html_string_render_the_same_document() {
    let (elements, string) = both_renders(DOC);
    assert_eq!(elements, string, "DocumentView and renderDocumentHtml differ");
    for expected in [
        "<h2 class=\"mt-6 mb-2 text-2xl font-semibold tracking-tight\">Field notes</h2>",
        "<a data-mention=\"person\" data-id=\"p1\" href=\"/people/p1\" class=\"rounded bg-accent px-1 font-medium text-accent-foreground no-underline\">@Alex Example</a>",
        "data-callout=\"warning\"",
        "<th class=\"border border-border bg-muted px-3 py-1.5 text-left font-semibold\">Site</th>",
        "<td class=\"border border-border px-3 py-1.5 align-top\">site-a</td>",
        "<figure data-figure=\"\" class=\"my-5\"><img src=\"/files/demo.png\" alt=\"A demo\" data-ref=\"uploads/demo.png\" class=\"w-full rounded-lg\"><figcaption class=\"mt-2 text-sm text-muted-foreground\">The demo site</figcaption>",
        "Photo: Example</p></figure>",
        "data-embed=\"product\" data-key=\"notebook\" href=\"/shop/notebook\"",
        "<strong class=\"block truncate font-semibold\">Field notebook</strong>",
        "data-recipe=\"soup\"",
        "Serves 4",
        "<input type=\"checkbox\" checked=\"\" disabled=\"\" class=\"mt-2 size-4 shrink-0 accent-primary\">",
        "&lt;tags&gt; &amp; &quot;quotes&quot;",
        "href=\"https://example.com/x?a=1&amp;b=2\"",
    ] {
        assert!(elements.contains(expected), "missing {expected}\nin:\n{elements}");
    }
    // The editor-only attribute never reaches a page.
    assert!(!elements.contains("contenteditable"), "{elements}");
}

/// Citations number by first appearance; a work cited twice keeps its
/// number; the references are collected once, after the document.
#[test]
fn citations_are_numbered_and_collected_at_the_end() {
    let (elements, _) = both_renders(DOC);
    let first = elements.find("href=\"#citation-1\" class=\"text-info no-underline\">[1]</a>").expect("first citation is [1]");
    let second = elements.find("href=\"#citation-2\" class=\"text-info no-underline\">[2]</a>").expect("second citation is [2]");
    assert!(first < second, "{elements}");
    assert_eq!(elements.matches(">[1]</a>").count(), 2, "the repeated work keeps [1]:\n{elements}");
    let refs = between(&elements, "<section data-references=\"citation\"", "</section>");
    assert!(refs.contains(">References</h2>"), "{refs}");
    assert!(refs.contains("<li id=\"citation-1\">Example, A., Sample, S. (2020). <cite>On samples</cite>. Journal of Examples. <a href=\"https://doi.org/10.5555/a\""), "{refs}");
    assert!(refs.contains("<li id=\"citation-2\">Sample, S. (2021). <cite>More samples</cite>. <a href=\"https://example.com/more\""), "{refs}");
    assert_eq!(refs.matches("<li ").count(), 2, "{refs}");
    let pullquote = elements.find("data-unknown-node=\"pullquote\"").unwrap();
    assert!(elements.find("data-references").unwrap() > pullquote, "references come after the document");
}

/// A node no extension knows is shown as what it is, with its content.
#[test]
fn an_unknown_node_keeps_its_content() {
    let (elements, _) = both_renders(DOC);
    assert!(
        elements.contains("data-unknown-node=\"pullquote\" class=\"my-1.5 rounded-md border border-dashed border-border px-2 text-muted-foreground\"><p class=\"my-1.5\">Kept, not dropped.</p></div>"),
        "{elements}"
    );
    // An unknown node inside a paragraph stays inline.
    let (inline, _) = both_renders(r#"{ type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "a " }, { type: "emoji", attrs: { name: "x" }, content: [{ type: "text", text: "wave" }] }] }] }"#);
    assert!(inline.contains("<span data-unknown-node=\"emoji\" class=\"my-1.5 rounded-md border border-dashed border-border px-2 text-muted-foreground\">wave</span>"), "{inline}");
}

/// What a stored document or an extension says cannot become script: no
/// script/iframe/style elements, no `on…` handlers, no `style`, no
/// `javascript:` URL — in elements and in the string alike.
#[test]
fn sanitization_drops_scripts_handlers_and_script_urls() {
    let evil = r#"import { defineExtension } from "zeb/ui/editor-extension";
export const evil = defineExtension({
  name: "evil",
  node: { group: "block", atom: true },
  render: (_node, r) => r.h("div", { class: "x", onclick: "alert(1)", onMouseOver: "alert(2)", style: "background:url(javascript:alert(3))", "data-ok": "yes" },
    r.h("script", {}, "alert(4)"),
    r.h("iframe", { src: "https://example.com" }),
    r.h("style", {}, "body{}"),
    r.h("img", { src: "javascript:alert(5)", onerror: "alert(6)", srcset: "x 1x" }),
    r.h("a", { href: " JaVaScRiPt:alert(7)", target: "_top" }, "link"),
    r.h("a", { href: "https://example.com", target: "_blank" }, "out"),
    r.h("marquee", { "aria-label": "kept" }, "text stays"),
  ),
});
"#;
    let page = r#"import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";
import { evil } from "@/shared/editor/extensions/evil";
const LIST = [...EXTENSIONS, evil];
const DOC = { type: "doc", content: [
  { type: "evil" },
  { type: "script", content: [{ type: "text", text: "alert(8)" }] },
  { type: "paragraph", content: [{ type: "text", text: "x", marks: [{ type: "link", attrs: { href: "javascript:alert(9)" } }] }] },
  { type: "image", attrs: { src: "javascript:alert(10)", alt: "\" onerror=\"alert(11)" } },
  { type: "person", attrs: { id: "p", label: "<img src=x onerror=alert(12)>", href: "javascript:alert(13)" } },
  { type: "product", attrs: { key: "k", snapshot: { title: "t", href: "vbscript:x", image: "javascript:alert(14)" } } }
] };
export default function Page() {
  return <main><section id="elements"><DocumentView doc={DOC} extensions={LIST} /></section><pre id="string">{renderDocumentHtml(DOC, { extensions: LIST })}</pre></main>;
}
"#;
    let html = render(page, &[("shared/editor/extensions/evil.tsx", evil)]);
    let elements = between(&html, "<section id=\"elements\"><div data-slot=\"document\">", "</div></section>").to_string();
    let string = unescape(between(&html, "<pre id=\"string\">", "</pre>"));
    assert_eq!(elements, string, "the two renders differ");
    let lower = elements.to_lowercase();
    for banned in ["<script", "<iframe", "<style", "onclick=\"", "onmouseover=\"", "onerror=\"", "style=\"", "javascript:", "vbscript:", "srcset", "_top", "<marquee"] {
        assert!(!lower.contains(banned), "{banned} survived:\n{elements}");
    }
    assert!(elements.contains("data-ok=\"yes\""), "{elements}");
    assert!(elements.contains("target=\"_blank\" rel=\"noopener noreferrer\""), "{elements}");
    assert!(elements.contains("<span aria-label=\"kept\">text stays</span>"), "{elements}");
    assert!(elements.contains("data-unknown-node=\"script\" class=\"my-1.5 rounded-md border border-dashed border-border px-2 text-muted-foreground\">alert(8)</div>"), "an unknown node is text, never markup:\n{elements}");
    assert!(elements.contains("@&lt;img src=x onerror=alert(12)&gt;"), "{elements}");
}

/// Extensions compose: names and types are unique, the engine's own types are
/// reserved, and the plain-text form speaks through each atom's `text`.
#[test]
fn extensions_compose_and_collisions_are_refused_by_name() {
    let page = r#"import { composeExtensions, defineExtension } from "zeb/ui/editor-extension";
import { documentText } from "zeb/ui/editor-render";
import { EXTENSIONS, recipe } from "@/shared/editor/extensions/index";
const DOC = { type: "doc", content: [
  { type: "paragraph", content: [{ type: "text", text: "Hi" }, { type: "person", attrs: { id: "p1", label: "Alex" } }, { type: "citation", attrs: { id: "x", label: "Example 2020" } }] },
  { type: "recipe", attrs: { key: "soup" } }
] };
function attempt(list) {
  try { composeExtensions(list); return "ok"; } catch (err) { return String(err.message); }
}
export default function Page() {
  const reg = composeExtensions(EXTENSIONS);
  return <main>
    <p id="names">{reg.list.map((ext) => ext.name).join(",")}</p>
    <p id="nodes">{Object.keys(reg.nodes).join(",")}</p>
    <p id="twice">{attempt([recipe, recipe])}</p>
    <p id="base">{attempt([defineExtension({ name: "mine", nodes: { paragraph: { spec: { group: "block" } } } })])}</p>
    <p id="clash">{attempt([recipe, defineExtension({ name: "other", nodes: { recipe: { spec: { group: "block" } } } })])}</p>
    <p id="raw">{attempt([{ name: "raw" }])}</p>
    <p id="badname">{(() => { try { defineExtension({ name: "Bad Name" }); return "ok"; } catch (err) { return err.message; } })()}</p>
    <p id="text">{documentText(DOC, { extensions: EXTENSIONS })}</p>
  </main>;
}
"#;
    let html = render(page, &[]);
    assert!(html.contains("<p id=\"names\">callout,table,figure,product,person,citation,recipe</p>"), "{html}");
    assert!(html.contains("<p id=\"nodes\">callout,table,table_row,table_header,table_cell,figure,product,person,citation,recipe</p>"), "{html}");
    assert!(html.contains("<p id=\"twice\">editor extension &quot;recipe&quot; is listed twice</p>"), "{html}");
    assert!(html.contains("<p id=\"base\">editor extension &quot;mine&quot; redefines &quot;paragraph&quot;</p>"), "{html}");
    assert!(html.contains("<p id=\"clash\">editor extension &quot;other&quot; redefines &quot;recipe&quot;</p>"), "{html}");
    assert!(html.contains("<p id=\"raw\">an editor extension must come from defineExtension</p>"), "{html}");
    assert!(html.contains("<p id=\"badname\">an editor extension needs a name"), "{html}");
    assert!(html.contains("<p id=\"text\">Hi@Alex [Example 2020] recipe soup</p>"), "{html}");
}

/// A shipped extension that needs a data source says so when it has none.
#[test]
fn a_picker_extension_without_a_source_is_refused() {
    let page = r#"import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";
function attempt(make) { try { make(); return "ok"; } catch (err) { return err.message; } }
export default function Page() {
  return <main><p id="m">{attempt(() => mentionExtension({ name: "person" }))}</p><p id="c">{attempt(() => citationExtension({ search: async () => [] }))}</p></main>;
}
"#;
    let html = render(page, &[]);
    assert!(html.contains("<p id=\"m\">mentionExtension &quot;person&quot; needs a route (or a search function) for its picker</p>"), "{html}");
    assert!(html.contains("<p id=\"c\">ok</p>"), "{html}");
}
