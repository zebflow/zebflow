//! Editor extensions whose data belongs to the project: mention kinds,
//! references to other documents, and a Potoru story block — rendered by the
//! real compiler and SSR engine. Zebflow supplies the mechanism; every route,
//! kind and look below is configured the way a project would.

use super::zeb_ui_editor::{between, render, render_with_script, unescape};

/// The project's extension list: two mention kinds with their own triggers
/// and sources (one drawn its own way), a slash-only kind, a reference link
/// and a reference card, and the Potoru block.
const SOURCES: &str = r#"import { mentionExtension } from "zeb/ui/editor-mention";
import { referenceExtension } from "zeb/ui/editor-reference";
import { potoruExtension } from "zeb/ui/editor-potoru";

export const SOURCES = [
  mentionExtension({ name: "person", trigger: "@", route: "/api/people/search" }),
  mentionExtension({
    name: "org", trigger: "+", route: "/api/orgs/search",
    render: (node, r) => r.h("a", { class: "font-semibold text-primary", "data-org": node.attrs.id, href: node.attrs.href }, node.attrs.label, r.h("small", { class: "ml-1 text-muted-foreground" }, (node.attrs.snapshot || {}).city || "")),
  }),
  mentionExtension({ name: "topic", trigger: null, route: "/api/topics/search" }),
  referenceExtension({ name: "article", route: "/api/articles/search" }),
  referenceExtension({ name: "page_card", variant: "card", route: "/api/pages/search" }),
  potoruExtension({ route: "/api/stories/search" }),
];
"#;

fn page(doc: &str, body: &str) -> String {
    format!(
        r#"import {{ DocumentView, renderDocumentHtml, documentText }} from "zeb/ui/editor-render";
import {{ SOURCES }} from "@/shared/editor/extensions/sources";
const DOC = {doc};
export default function Page() {{
  return <main>{body}<section id="elements"><DocumentView doc={{DOC}} extensions={{SOURCES}} /></section><pre id="string">{{renderDocumentHtml(DOC, {{ extensions: SOURCES }})}}</pre><p id="text">{{documentText(DOC, {{ extensions: SOURCES }})}}</p></main>;
}}
"#
    )
}

/// Elements, the HTML string (which must be the same bytes) and the plain text.
fn renders(doc: &str) -> (String, String) {
    let html = render(&page(doc, ""), &[("shared/editor/extensions/sources.tsx", SOURCES)]);
    let elements = between(&html, "<section id=\"elements\"><div data-slot=\"document\">", "</div></section>").to_string();
    assert_eq!(elements, unescape(between(&html, "<pre id=\"string\">", "</pre>")), "DocumentView and renderDocumentHtml differ");
    (elements, unescape(between(&html, "<p id=\"text\">", "</p>")))
}

/// Two kinds of mention side by side, each with its own trigger and look;
/// nothing about people or organizations is built in.
#[test]
fn mention_kinds_are_the_projects_own() {
    let (elements, text) = renders(r#"{ type: "doc", content: [{ type: "paragraph", content: [
      { type: "person", attrs: { id: "p1", label: "Alex Example", href: "/people/p1" } },
      { type: "text", text: " of " },
      { type: "org", attrs: { id: "o1", label: "Example Lab", href: "/orgs/o1", snapshot: { city: "Sampleton" } } },
      { type: "text", text: " on " },
      { type: "topic", attrs: { id: "t1", label: "maps" } }
    ] }] }"#);
    assert!(elements.contains("<a data-mention=\"person\" data-id=\"p1\" href=\"/people/p1\" class=\"rounded bg-accent px-1 font-medium text-accent-foreground no-underline\">@Alex Example</a>"), "{elements}");
    assert!(elements.contains("<a data-org=\"o1\" href=\"/orgs/o1\" class=\"font-semibold text-primary\">Example Lab<small class=\"ml-1 text-muted-foreground\">Sampleton</small></a>"), "{elements}");
    assert!(elements.contains("data-mention=\"topic\" data-id=\"t1\" class=\"rounded bg-accent px-1 font-medium text-accent-foreground no-underline\">maps</span>"), "a slash-only kind has no prefix:\n{elements}");
    assert_eq!(text, "@Alex Example of +Example Lab on maps");
}

/// A trigger opens one picker: two kinds on the same character, or a
/// trigger that is not one character, are refused when the list is built.
#[test]
fn mention_kinds_need_their_own_trigger() {
    let probe = r#"import { composeExtensions } from "zeb/ui/editor-extension";
import { mentionExtension } from "zeb/ui/editor-mention";
import { referenceExtension } from "zeb/ui/editor-reference";
function attempt(make) { try { make(); return "ok"; } catch (err) { return err.message; } }
const search = async () => [];
export default function Page() {
  return <main>
    <p id="same">{attempt(() => composeExtensions([mentionExtension({ name: "person", search }), mentionExtension({ name: "org", search })]))}</p>
    <p id="long">{attempt(() => mentionExtension({ name: "org", trigger: "@@", search }))}</p>
    <p id="ok">{attempt(() => composeExtensions([mentionExtension({ name: "person", search }), mentionExtension({ name: "org", trigger: "+", search }), mentionExtension({ name: "tag", trigger: null, search })]))}</p>
    <p id="ref">{attempt(() => referenceExtension({ name: "article" }))}</p>
    <p id="variant">{attempt(() => referenceExtension({ name: "article", search, variant: "tile" }))}</p>
  </main>;
}
"#;
    let html = render(probe, &[]);
    assert!(html.contains("<p id=\"same\">editor extensions &quot;person&quot; and &quot;org&quot; share the trigger &quot;@&quot;</p>"), "{html}");
    assert!(html.contains("<p id=\"long\">mentionExtension &quot;org&quot;: trigger is one character, or null</p>"), "{html}");
    assert!(html.contains("<p id=\"ok\">ok</p>"), "{html}");
    assert!(html.contains("<p id=\"ref\">referenceExtension &quot;article&quot; needs a route (or a search function) for its picker</p>"), "{html}");
    assert!(html.contains("<p id=\"variant\">referenceExtension &quot;article&quot;: variant is &quot;link&quot; or &quot;card&quot;</p>"), "{html}");
}

/// A reference renders from what it stored — a link inline, or a small card —
/// and its URL passes the same allowlist as every other link.
#[test]
fn references_render_from_their_snapshot() {
    let (elements, text) = renders(r#"{ type: "doc", content: [
      { type: "paragraph", content: [
        { type: "text", text: "See " },
        { type: "article", attrs: { key: "a-1", label: "Field guide", href: "/articles/field-guide", snapshot: { description: "How we survey" } } },
        { type: "text", text: " and " },
        { type: "article", attrs: { key: "a-2", label: "Draft", href: null } },
        { type: "article", attrs: { key: "a-3", label: "Bad", href: "javascript:alert(1)" } }
      ] },
      { type: "page_card", attrs: { key: "pg-1", label: "Site map", href: "/pages/map", snapshot: { description: "Every page", meta: "Updated weekly" } } }
    ] }"#);
    assert!(elements.contains("<a data-reference=\"article\" data-key=\"a-1\" title=\"How we survey\" href=\"/articles/field-guide\" class=\"text-info underline underline-offset-4\">Field guide</a>"), "{elements}");
    assert!(elements.contains("<span data-reference=\"article\" data-key=\"a-2\" class=\"text-info underline underline-offset-4\">Draft</span>"), "{elements}");
    assert!(elements.contains("data-key=\"a-3\" href=\"#\""), "{elements}");
    assert!(!elements.contains("javascript:"), "{elements}");
    assert!(elements.contains("<a data-reference=\"page_card\" data-key=\"pg-1\" title=\"Every page\" href=\"/pages/map\" class=\"my-3 block rounded-md border border-border bg-card px-3 py-2 text-card-foreground no-underline hover:bg-accent\"><strong class=\"block truncate text-sm font-semibold\">Site map</strong><span class=\"block truncate text-xs text-muted-foreground\">Every page</span><small class=\"block text-xs text-muted-foreground\">Updated weekly</small></a>"), "{elements}");
    assert_eq!(text, "See Field guide and Draft Bad Site map");
}

fn attr<'a>(html: &'a str, name: &str) -> &'a str {
    between(html, &format!("{name}=\""), "\"")
}

/// The block's placeholder is PotoPlayer's: same library, same wrapper, the
/// same `data-config` bytes for the same settings — so it hydrates into the
/// player. Its address passes the URL allowlist; a block with no address
/// renders nothing on the page.
#[test]
fn a_potoru_block_renders_potoplayers_placeholder() {
    let doc = r#"{ type: "doc", content: [
      { type: "potoru", attrs: { key: "intro", src: "/_files/stories/intro.poto", mode: "slide", still: true, time: 1.5, snapshot: { title: "Intro", poster: "/_files/stories/intro.png" } } },
      { type: "potoru", attrs: { key: "bad", src: "javascript:alert(1)", mode: "rogue", still: false, time: 9 } },
      { type: "potoru", attrs: { key: "empty", src: "" } }
    ] }"#;
    let player = r#"<section id="player"><PotoPlayer src="/_files/stories/intro.poto" controls still time={1.5} mode="slide" /></section>"#;
    let html = render(&format!("import {{ PotoPlayer }} from \"zeb/potoru\";\n{}", page(doc, player)), &[("shared/editor/extensions/sources.tsx", SOURCES)]);
    let stub = between(&html, "<section id=\"player\">", "</section>");
    let elements = between(&html, "<section id=\"elements\"><div data-slot=\"document\">", "</div></section>");
    assert_eq!(attr(elements, "data-config"), attr(stub, "data-config"), "\nblock: {elements}\nstub: {stub}");
    assert_eq!(attr(elements, "data-zeb-lib"), "potoru");
    assert_eq!(attr(elements, "data-zeb-wrapper"), "PotoPlayer");
    assert_eq!(attr(stub, "data-zeb-wrapper"), "PotoPlayer");
    assert!(elements.contains("<figure data-potoru-block=\"potoru\" data-key=\"intro\" class=\"my-4\"><div data-zeb-lib=\"potoru\""), "{elements}");
    assert!(elements.contains("><img src=\"/_files/stories/intro.png\" alt=\"Intro\" class=\"block w-full\"></div><figcaption class=\"mt-2 text-sm text-muted-foreground\">Intro</figcaption></figure>"), "the poster sits inside the placeholder:\n{elements}");
    // A hostile address becomes "#"; an unknown mode and a time without still are left out.
    assert!(elements.contains("data-config=\"{&quot;src&quot;:&quot;#&quot;,&quot;controls&quot;:true}\""), "{elements}");
    assert!(!elements.contains("data-key=\"empty\""), "{elements}");
    assert!(!elements.contains("javascript:"), "{elements}");
}

/// The page loads `zeb/potoru` only where a story can appear: a page whose
/// server HTML holds a block gets the on-demand loader (never an up-front
/// import), and a page with documents but no Potoru extension gets nothing.
#[test]
fn only_a_page_that_can_show_a_story_loads_potoru() {
    let url = "/assets/libraries/zeb/potoru/0.1/runtime/potoru.bundle.mjs";
    let with_block = page(r#"{ type: "doc", content: [{ type: "potoru", attrs: { key: "intro", src: "/_files/stories/intro.poto" } }] }"#, "");
    let (html, script) = render_with_script(&with_block, &[("shared/editor/extensions/sources.tsx", SOURCES)]);
    assert!(html.contains("data-zeb-lib=\"potoru\""), "{html}");
    assert!(script.contains(&format!("import('{url}')")), "the loader is on the page");
    assert!(!script.contains(&format!("await import('{url}')")), "and does not hold up hydration");

    // No block yet, but the page can draw one (the editor): the loader waits for it.
    let empty = page(r#"{ type: "doc", content: [] }"#, "");
    let (html, script) = render_with_script(&empty, &[("shared/editor/extensions/sources.tsx", SOURCES)]);
    assert!(!html.contains("data-zeb-lib=\"potoru\""), "{html}");
    assert!(script.contains(&format!("import('{url}')")), "a page that can draw a story gets the loader");

    let without = r#"import { DocumentView } from "zeb/ui/editor-render";
import { mentionExtension } from "zeb/ui/editor-mention";
const LIST = [mentionExtension({ name: "person", route: "/api/people/search" })];
const DOC = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "No stories here." }] }] };
export default function Page() { return <main><DocumentView doc={DOC} extensions={LIST} /></main>; }
"#;
    let (html, script) = render_with_script(without, &[]);
    assert!(html.contains("No stories here."), "{html}");
    assert!(!script.contains(url), "a page without a story never asks for zeb/potoru");
}
