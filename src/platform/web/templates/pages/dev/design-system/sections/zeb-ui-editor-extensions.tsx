import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";
import { defineExtension } from "zeb/ui/editor-extension";
import { calloutExtension } from "zeb/ui/editor-callout";
import { tableExtension } from "zeb/ui/editor-table";
import { figureExtension } from "zeb/ui/editor-figure";
import { embedExtension } from "zeb/ui/editor-embed";
import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";
import { referenceExtension } from "zeb/ui/editor-reference";
import { EditorPicker } from "zeb/ui/editor-picker";
import { EditorNodePanel } from "zeb/ui/editor-panel";
import { CodeBlock } from "zeb/ui/code-block";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

// The gallery has no project routes, so the pickers search a fixed list.
// A project passes `route: "/…"` instead and its pipeline answers.
const PEOPLE = [
  { id: "p1", label: "Alex Example", href: "/people/p1", snapshot: { description: "Field notes" } },
  { id: "p2", label: "Sam Sample", href: "/people/p2", snapshot: { description: "Maps and charts" } },
];
const WORKS = [
  { id: "10.5555/example.1", label: "Example 2020", href: "https://doi.org/10.5555/example.1", snapshot: { authors: ["Example, A."], year: 2020, title: "On sample documents", container: "Journal of Examples", doi: "10.5555/example.1" } },
];
const ORGS = [{ id: "o1", label: "Example Lab", href: "/orgs/o1", snapshot: { description: "Sampleton" } }];
const ARTICLES = [
  { id: "a-1", label: "Field guide", href: "/articles/field-guide", snapshot: { description: "How we survey" } },
  { id: "a-2", label: "Annual report", href: "/articles/annual", snapshot: { description: "The year in numbers" } },
];
const PRODUCTS = [
  { id: "notebook", label: "Field notebook", href: "/shop/notebook", snapshot: { description: "Ninety-six pages, dot grid.", meta: "In stock" } },
];
const listSearch = (items) => async (query) => items.filter((item) => item.label.toLowerCase().includes(query.toLowerCase()));

// A project extension in one declaration: a highlighted mark.
const highlight = defineExtension({
  name: "highlight",
  mark: {},
  render: (_mark, r) => r.h("mark", { class: "rounded bg-warning/30 px-0.5 text-foreground" }, r.content),
  parse: [{ tag: "mark" }],
});

const EXTENSIONS = [
  calloutExtension(),
  tableExtension(),
  figureExtension(),
  embedExtension({ name: "product", label: "Product", search: listSearch(PRODUCTS) }),
  mentionExtension({ name: "person", search: listSearch(PEOPLE) }),
  mentionExtension({ name: "org", trigger: "+", label: "Organization", search: listSearch(ORGS) }),
  citationExtension({ search: listSearch(WORKS) }),
  referenceExtension({ name: "article", label: "Link to article", search: listSearch(ARTICLES) }),
  highlight,
];

const text = (value, marks) => (marks ? { type: "text", text: value, marks } : { type: "text", text: value });
const SAMPLE = {
  type: "doc",
  content: [
    { type: "paragraph", content: [text("Written by "), { type: "person", attrs: { id: "p1", label: "Alex Example", href: "/people/p1", snapshot: null } }, text(" — type "), text("@", [{ type: "code" }]), text(" for a person, "), text("+", [{ type: "code" }]), text(" for an organization like "), { type: "org", attrs: { id: "o1", label: "Example Lab", href: "/orgs/o1", snapshot: null } }, text(", "), text("/link", [{ type: "code" }]), text(" for an article like "), { type: "article", attrs: { key: "a-1", label: "Field guide", href: "/articles/field-guide", snapshot: ARTICLES[0].snapshot } }, text(", "), text("/cite", [{ type: "code" }]), text(" for a work"), { type: "citation", attrs: { id: "10.5555/example.1", label: "Example 2020", href: null, snapshot: WORKS[0].snapshot } }, text(", and "), text("highlight", [{ type: "highlight" }]), text(" is a mark from one defineExtension.")] },
    { type: "callout", attrs: { icon: "💡", tone: "info" }, content: [{ type: "paragraph", content: [text("Callouts, tables, figures, embeds, mentions and citations are extensions. Put the caret in one to edit it in its panel.")] }] },
    { type: "table", content: [
      { type: "table_row", content: [{ type: "table_header", content: [text("Block")] }, { type: "table_header", content: [text("Stored as")] }] },
      { type: "table_row", content: [{ type: "table_cell", content: [text("Embed")] }, { type: "table_cell", content: [text("key + snapshot")] }] },
    ] },
    { type: "figure", attrs: { src: "", alt: "", ref: null, credit: "Photo: the gallery" }, content: [text("A figure: image, caption, credit.")] },
    { type: "product", attrs: { key: "notebook", snapshot: { title: "Field notebook", description: "Ninety-six pages, dot grid.", href: "/shop/notebook", meta: "In stock" } } },
  ],
};

export default function ZebUiEditorExtensionsSection() {
  const [doc, setDoc] = useState(SAMPLE);
  return (
    <div>
      <SectionHeading title="zeb/ui · Editor extensions" description="Blocks, inline nodes and marks a project adds to the editor. One declaration edits, stores (JSON) and renders (DocumentView, renderDocumentHtml — the same on the server and in the browser)." />
      <Entry
        name="Extensions"
        file="zeb/ui/editor-extension"
        description="Left: the editor with every shipped extension and one defined inline. Right: DocumentView with the same list — citations are numbered and collected under References."
        code={`import { calloutExtension } from "zeb/ui/editor-callout";
import { tableExtension } from "zeb/ui/editor-table";
import { figureExtension } from "zeb/ui/editor-figure";
import { embedExtension } from "zeb/ui/editor-embed";
import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";
import { referenceExtension } from "zeb/ui/editor-reference";
import { potoruExtension } from "zeb/ui/editor-potoru";

// shared/editor/extensions/index.tsx — built once, used by the editor and every page that renders
export const EXTENSIONS = [
  calloutExtension(), tableExtension(), figureExtension(),
  embedExtension({ name: "product", label: "Product", route: "/api/products/search" }),
  mentionExtension({ name: "person", route: "/api/people/search" }),   // answers [{ id, label, href, snapshot }]
  mentionExtension({ name: "org", trigger: "+", route: "/api/orgs/search" }),   // a second kind: own trigger, own route
  citationExtension({ route: "/api/works/search" }),
  referenceExtension({ name: "article", route: "/api/articles/search" }),   // "/link": search the project's documents
  potoruExtension(),   // a .poto story block; zeb/potoru loads only on pages that hold one
];

<Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} />
<DocumentView doc={doc} extensions={EXTENSIONS} />
renderDocumentHtml(doc, { extensions: EXTENSIONS })`}
      >
        <div className="grid gap-6 lg:grid-cols-2">
          <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} />
          <div className="rounded-lg border border-dashed border-border p-4">
            <div className="mb-3 font-mono text-[0.65rem] uppercase tracking-wider text-muted-foreground">DocumentView — with the same extensions</div>
            <DocumentView doc={doc} extensions={EXTENSIONS} />
          </div>
        </div>
        <div className="mt-6">
          <CodeBlock language="tsx" title="renderDocumentHtml(doc, { extensions })" maxHeight="16rem" code={renderDocumentHtml(doc, { extensions: EXTENSIONS })} />
        </div>
      </Entry>
      <Entry name="EditorPicker" file="zeb/ui/editor-picker" description="The list behind @, + and /cite: asks the extension's search (a project route), shows its error if it fails. Used by Editor; type @ in the editor above." code={`picker: { search: routeSearch("/api/people/search"), toAttrs: (item) => ({ id: item.id, label: item.label }) }`}>
        <p className="text-sm text-muted-foreground">{typeof EditorPicker === "function" ? "Opened by the editor above." : ""}</p>
      </Entry>
      <Entry name="EditorNodePanel" file="zeb/ui/editor-panel" description="The panel for the node under the caret: an extension's fields (attrs, dotted paths) and actions (add a table row…)." code={`fields: [{ name: "credit", label: "Credit" }, { name: "snapshot.title", label: "Title" }],
actions: [{ label: "Add row", run: (editor) => editor.exec("tableAddRow") }]`}>
        <p className="text-sm text-muted-foreground">{typeof EditorNodePanel === "function" ? "Put the caret in the table or the figure above." : ""}</p>
      </Entry>
    </div>
  );
}
