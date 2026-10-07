//! `<DocumentHtml html>` on a page, through the real compiler and SSR engine:
//! stored document HTML is shown — sanitized, exactly as `<DocumentView>`
//! shows the document it came from — while raw HTML stays refused.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{ReactiveWebEngine, ReactiveWebOptions, RweReactiveWebEngine, TemplateOptions, TemplateSource};

use super::zeb_ui_editor::{between, render, render_with_input};

/// The project's extensions: the shipped figure and Potoru blocks.
const EXTENSIONS: &str = r#"import { figureExtension } from "zeb/ui/editor-figure";
import { calloutExtension } from "zeb/ui/editor-callout";
import { potoruExtension } from "zeb/ui/editor-potoru";

export const STORY_EXTENSIONS = [figureExtension(), calloutExtension(), potoruExtension()];
"#;

const DOC: &str = r#"{ type: "doc", content: [
  { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Field notes" }] },
  { type: "paragraph", content: [
    { type: "text", text: "Survey " },
    { type: "text", text: "north", marks: [{ type: "bold" }] },
    { type: "text", text: " <tags> & \"quotes\"", marks: [{ type: "link", attrs: { href: "https://example.com/x?a=1&b=2", title: null } }] }
  ] },
  { type: "figure", attrs: { src: "/_files/uploads/demo.png", alt: "A demo", ref: "uploads/demo.png", credit: "Photo: Example" }, content: [{ type: "text", text: "The demo site" }] },
  { type: "callout", attrs: { icon: "!", tone: "warning" }, content: [{ type: "paragraph", content: [{ type: "text", text: "Mind the step." }] }] },
  { type: "potoru", attrs: { key: "intro", src: "/_files/stories/intro.poto", snapshot: { title: "Intro", poster: "/_files/posters/intro.png" } } },
  { type: "todo_list", content: [{ type: "todo_item", attrs: { checked: true }, content: [{ type: "paragraph", content: [{ type: "text", text: "Done" }] }] }] },
  { type: "code_block", attrs: { language: "js" }, content: [{ type: "text", text: "const a = 1 < 2;" }] }
] }"#;

/// The HTML `renderDocumentHtml` stores, shown with `<DocumentHtml>`, is the
/// same bytes `<DocumentView>` prints for the document — the Potoru
/// placeholder included, so the page hydrates it as it would the document.
#[test]
fn stored_html_shows_exactly_as_the_document_does() {
    let page = format!(
        r#"import {{ DocumentView, renderDocumentHtml }} from "zeb/ui/editor-render";
import {{ DocumentHtml }} from "zeb/ui/editor-html";
import {{ STORY_EXTENSIONS }} from "@/shared/editor/extensions/story";
const DOC = {DOC};
export default function Page() {{
  return <main><section id="view"><DocumentView doc={{DOC}} extensions={{STORY_EXTENSIONS}} /></section><section id="html"><DocumentHtml html={{renderDocumentHtml(DOC, {{ extensions: STORY_EXTENSIONS }})}} /></section></main>;
}}
"#
    );
    let html = render(&page, &[("shared/editor/extensions/story.tsx", EXTENSIONS)]);
    let view = between(&html, "<section id=\"view\">", "</section>");
    let shown = between(&html, "<section id=\"html\">", "</section>");
    assert_eq!(shown, view, "DocumentHtml and DocumentView differ");
    assert!(view.contains("data-zeb-lib=\"potoru\" data-zeb-wrapper=\"PotoPlayer\""), "{view}");
    assert!(view.contains("<figure") && view.contains("Photo: Example"), "{view}");
}

/// A display page: the HTML arrives from the pipeline, the page imports
/// `DocumentHtml` and nothing else. A tampered row is shown without its
/// script, handler, iframe, style or `javascript:` link. The Potoru library
/// is loaded because, and only when, a block is in the HTML.
#[test]
fn a_display_page_sanitizes_stored_html_and_loads_the_player_only_for_a_block() {
    let page = r#"import { DocumentHtml } from "zeb/ui/editor-html";
export default function Page(input) {
  return <main><DocumentHtml html={input.body_html} className="prose" /></main>;
}
"#;
    let tampered = concat!(
        r#"<p class="my-1.5" onclick="steal()" style="color:red">Hi<script>alert(1)</script></p>"#,
        r#"<img src="/_files/a.png" onerror="alert(1)"><a href="javascript:alert(1)">go</a>"#,
        r#"<iframe src="https://example.com/x"></iframe><style>p{}</style><svg onload="alert(1)"></svg>"#,
    );
    let (html, script) = render_with_input(page, &[], json!({ "body_html": tampered }));
    let main = between(&html, "<main>", "</main>");
    assert_eq!(
        main,
        r##"<div data-slot="document" class="prose"><p class="my-1.5">Hi</p><img src="/_files/a.png"><a href="#">go</a></div>"##
    );
    assert!(!script.contains("[data-zeb-lib=\"potoru\"]"), "no block, no player loader");

    let block = r#"<figure data-potoru-block="potoru" data-key="intro" class="my-4"><div data-zeb-lib="potoru" data-zeb-wrapper="PotoPlayer" data-config="{&quot;src&quot;:&quot;/_files/stories/intro.poto&quot;,&quot;controls&quot;:true}" class="relative block w-full overflow-hidden rounded-lg bg-muted"></div></figure>"#;
    let (html, script) = render_with_input(page, &[], json!({ "body_html": block }));
    assert!(between(&html, "<main>", "</main>").contains(block), "{html}");
    assert!(script.contains("[data-zeb-lib=\"potoru\"]"), "a block on the page loads the player:\n{script}");
}

/// The door is `DocumentHtml`, not raw HTML: a page that sets HTML itself is
/// still refused when it compiles.
#[test]
fn raw_html_on_a_page_stays_refused() {
    let compile = |page: &str| {
        let tmp = tempfile::Builder::new().prefix("zeb-ui-editor-html-").tempdir().expect("tempdir");
        let page_path = tmp.path().join("page.tsx");
        std::fs::write(&page_path, page).unwrap();
        let mut library_roots = BTreeMap::new();
        library_roots.insert(
            "zeb/ui".to_string(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("blessed/source-libraries/ui/0.1/src"),
        );
        RweReactiveWebEngine
            .compile_template(
                &TemplateSource { id: "raw.page".to_string(), source_path: Some(page_path), markup: page.to_string() },
                &NoopLanguageEngine,
                &ReactiveWebOptions {
                    templates: TemplateOptions { library_roots, template_root: Some(tmp.path().to_path_buf()), style_entries: Vec::new() },
                    ..Default::default()
                },
            )
            .map(|_| ())
    };
    let raw = r#"export default function Page(input) {
  return <main><div dangerouslySetInnerHTML={{ __html: input.body_html }} /></main>;
}
"#;
    let refused = compile(raw).expect_err("raw HTML is refused");
    assert!(refused.message.contains("dangerouslySetInnerHTML is blocked by security policy"), "{}: {}", refused.code, refused.message);
    let door = r#"import { DocumentHtml } from "zeb/ui/editor-html";
export default function Page(input) {
  return <main><DocumentHtml html={input.body_html} /></main>;
}
"#;
    compile(door).expect("a page using DocumentHtml compiles");
}
