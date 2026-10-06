import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";
import { defineExtension } from "zeb/ui/editor-extension";
import { guardComponent } from "zeb/ui/editor-component";
import { PotoruLibraries } from "zeb/ui/editor-potoru-libraries";
import { CodeBlock } from "zeb/ui/code-block";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

const STATUS_TONES = {
  draft: "bg-muted text-muted-foreground",
  review: "bg-warning/20 text-warning",
  done: "bg-success/20 text-success",
};

// The published look: printed by the page's SSR, hydrated with it, and the
// same bytes from renderDocumentHtml.
function StatusBadge({ attrs, options }) {
  return <span data-status={attrs.state} className={`rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_TONES[attrs.state] || STATUS_TONES.draft}`}>{options.labels[attrs.state] || attrs.state}</span>;
}

// The editor's look: the badge, and a select that changes the node.
function StatusEditor(props) {
  const { attrs, options, update, readOnly } = props;
  return (
    <span className="inline-flex items-center gap-1 align-middle">
      <StatusBadge {...props} />
      {readOnly ? null : (
        <select aria-label="Status" value={attrs.state} onChange={(e) => update({ state: e.target.value })} className="h-6 rounded border border-input bg-background px-1 text-xs">
          {Object.keys(options.labels).map((state) => <option key={state} value={state}>{options.labels[state]}</option>)}
        </select>
      )}
    </span>
  );
}

function statusExtension({ labels = { draft: "Draft", review: "In review", done: "Done" } } = {}) {
  return defineExtension({
    name: "status",
    node: { group: "inline", inline: true, atom: true, attrs: { state: { default: "draft" } } },
    options: { labels },
    component: StatusBadge,
    editComponent: StatusEditor,
    text: (node) => labels[node.attrs.state] || node.attrs.state,
    insert: [{ label: "Status", hint: "A status badge", keys: "status badge", run: (api) => api.insert("status") }],
  });
}

const EXTENSIONS = [statusExtension()];
const SAMPLE = {
  type: "doc",
  content: [{ type: "paragraph", content: [
    { type: "text", text: "Survey of site-a " },
    { type: "status", attrs: { state: "review" } },
    { type: "text", text: " — change it with its select, or type /status for another." },
  ] }],
};

export default function ZebUiEditorComponentsSection() {
  const [doc, setDoc] = useState(SAMPLE);
  const [libraries, setLibraries] = useState(["/_files/libs/basic.potolib"]);
  return (
    <div>
      <SectionHeading title="zeb/ui · Editor components" description="An extension may draw its node with Zeb React components: one for the page (server-rendered, hydrated, sanitized) and one for the editor (a live node view)." />
      <Entry
        name="component / editComponent"
        file="zeb/ui/editor-component"
        description="Left: the editor draws the node with editComponent and changes it with update(). Right: DocumentView draws it with component; renderDocumentHtml prints the same bytes."
        code={`defineExtension({
  name: "status",
  node: { group: "inline", inline: true, atom: true, attrs: { state: { default: "draft" } } },
  options: { labels },          // handed to both components
  component: StatusBadge,       // ({ attrs, options }) => <span …/>       — the page
  editComponent: StatusEditor,  // ({ attrs, options, update, selected, readOnly }) — the editor
})`}
      >
        <div className="grid gap-6 lg:grid-cols-2">
          <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} minHeight="6rem" />
          <div className="rounded-lg border border-dashed border-border p-4">
            <div className="mb-3 font-mono text-[0.65rem] uppercase tracking-wider text-muted-foreground">DocumentView — component</div>
            <DocumentView doc={doc} extensions={EXTENSIONS} />
          </div>
        </div>
        <div className="mt-6">
          <CodeBlock language="html" title="renderDocumentHtml(doc, { extensions })" maxHeight="10rem" code={renderDocumentHtml(doc, { extensions: EXTENSIONS })} />
        </div>
        <p className="mt-2 text-xs text-muted-foreground">{typeof guardComponent === "function" ? "A component's page output passes the document allowlist (guardComponent)." : ""}</p>
      </Entry>
      <Entry
        name="PotoruLibraries"
        file="zeb/ui/editor-potoru-libraries"
        description="The library list under a Potoru story block in the editor: the page's defaults (read-only, struck through when the block replaces one) and the block's own. Only https:// or a path on this site is accepted."
        code={`potoruExtension({ libraries: ["https://example.com/libs/basic.potolib"] })   // defaults for every block`}
      >
        <PotoruLibraries defaults={["https://example.com/libs/basic.potolib", "https://example.com/libs/shapes.potolib"]} libraries={libraries} onChange={setLibraries} />
        <p className="mt-2 font-mono text-xs text-muted-foreground">block: {JSON.stringify(libraries)}</p>
      </Entry>
    </div>
  );
}
