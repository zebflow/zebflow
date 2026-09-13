import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";
import { CodeBlock } from "zeb/ui/code-block";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

const SAMPLE = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "A page about pipelines" }] },
    { type: "paragraph", content: [{ type: "text", text: "Type " }, { type: "text", marks: [{ type: "code" }], text: "/" }, { type: "text", text: " for blocks, select text for the toolbar, or write Markdown shortcuts. Drag the " }, { type: "text", marks: [{ type: "bold" }], text: "⋮⋮" }, { type: "text", text: " handle to move a block." }] },
    { type: "todo_list", content: [
      { type: "todo_item", attrs: { checked: true }, content: [{ type: "paragraph", content: [{ type: "text", text: "Register the webhook" }] }] },
      { type: "todo_item", attrs: { checked: false }, content: [{ type: "paragraph", content: [{ type: "text", text: "Approve the first member" }] }] },
    ] },
    { type: "callout", attrs: { icon: "💡" }, content: [{ type: "paragraph", content: [{ type: "text", text: "The document is JSON. What you see on the right is " }, { type: "text", marks: [{ type: "code" }], text: "<DocumentView>" }, { type: "text", text: " rendering the same JSON — no editor involved." }] }] },
    { type: "code_block", attrs: { language: "tsx" }, content: [{ type: "text", text: 'import { Editor } from "zeb/ui/editor";\n\n<Editor value={doc} onChange={setDoc} />' }] },
  ],
};

export default function ZebUiEditorSection() {
  const [doc, setDoc] = useState(SAMPLE);
  return (
    <div>
      <SectionHeading title="zeb/ui · Editor" description="A Notion-style block editor on ProseMirror. The value is a document (JSON); DocumentView and renderDocumentHtml render it anywhere." />
      <Entry
        name="Editor"
        file="zeb/ui/editor"
        description="Left: the editor. Right: the same document rendered by DocumentView, live. Slash menu, bubble toolbar, Markdown shortcuts, to-dos, callouts, code blocks, drag handle."
        code={`import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";

const [doc, setDoc] = useState(EMPTY);
async function uploadImage(file) {              // the page decides where images go
  const form = new FormData();
  form.append("file", file);
  const { saved } = await (await fetch("/wh/o/p/upload", { method: "POST", body: form })).json();   // n.fs.save
  return { src: \`/files/o/p/\${saved.path}\`, ref: saved.path, alt: file.name };
}

<Editor value={doc} onChange={setDoc} uploadImage={uploadImage} />
<DocumentView doc={doc} />            // render it
renderDocumentHtml(doc)               // or store body_html`}
      >
        <div className="grid gap-6 lg:grid-cols-2">
          <Editor value={doc} onChange={setDoc} />
          <div className="rounded-lg border border-dashed border-border p-4">
            <div className="mb-3 font-mono text-[0.65rem] uppercase tracking-wider text-muted-foreground">DocumentView — rendered from the JSON</div>
            <DocumentView doc={doc} />
          </div>
        </div>
        <div className="mt-6 grid gap-4 lg:grid-cols-2">
          <CodeBlock language="json" title="doc (what you store)" maxHeight="16rem" code={JSON.stringify(doc, null, 2)} />
          <CodeBlock language="tsx" title="renderDocumentHtml(doc)" maxHeight="16rem" code={renderDocumentHtml(doc)} />
        </div>
      </Entry>
    </div>
  );
}
