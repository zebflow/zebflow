---
name: zebflow-files-editor
description: Uploads, images, files and rich text in a Zebflow project — FileRef, fs.file.put and fs.image.thumbnail, public vs private URLs, the zeb/ui editor and how its document is stored and rendered. Use before building an upload form, an image field, a media library, or any page with authored rich content.
license: MIT
metadata:
  version: "1"
---

# Files and rich text

Two rules carry this whole area: **bytes travel as FileRefs, never inline**,
and **rich text is a JSON document, HTML is derived from it**. Facts:
`help(topic="pipeline/authoring")` (FileRef), the `fs.*` nodes in
`pipeline/nodes`, `help(topic="web/ui")` (the editor).

## Uploads

1. A `<form method="post" enctype="multipart/form-data">` with
   `<input type="file" name="photo">`, or a `fetch` with `FormData`.
2. The webhook delivers the file as `input.webhook.files.photo` (anywhere
   later `$trigger.files.photo`) — a FileRef with `lifecycle: temporary`. It
   is discarded after the run unless a node keeps it.
3. Keep it: `fs.file.put --from "{{ input.webhook.files.photo }}" --folder uploads --accept image --max-size 10MB`
   checks the file by its content (the claimed type must agree, `--accept`
   must allow it) and adds `file` — a durable FileRef — to the payload; the
   form's other fields (`input.webhook.body.caption`) are still there.
4. Derive what you need: `fs.image.thumbnail --from "{{ input.file }}" --width 320 --height 320 --fit cover --format webp --folder thumbs`
   reads the file `--from` names and adds `image`, another FileRef.
5. Store the **store key** (`input.file.ref`) in your table, never a URL — a
   URL depends on the host and on what the owner exposed.

```
| trigger.webhook --route /api/upload --method POST --auth jwt --credential jwt_main
| fs.file.put --from "{{ input.webhook.files.file }}" --folder uploads --accept image --max-size 10MB
| web.response.send --body "{{ { ref: input.file.ref } }}"
```

Every `fs.*` node names its file with `--from` (a FileRef, an upload or a
store key) and every writer takes the same destination flags (`--store`,
`--folder`, `--filename`, `--path`, `--on-conflict`); each node's page has
the rest (`help(topic="pipeline/nodes/fs.file.put")`).

## Where a file is reachable

| Stored under | Path a page writes | Who |
|---|---|---|
| a folder exposed `public_read` in Studio → Files | the project's file host, `<project>.<owner>.fs.localhost/<path>` on a dev machine | anyone; inert — usable by an `<img>` or an `og:image`, never run as a page |
| a folder exposed `public_execute` | `/` on each address in its `serve` | anyone; served as a site, scripts running |
| anything else | nowhere without sign-in; the Studio reads it at `files/object?ref=…` | a signed-in session with files access |

No node answers a URL — a FileRef carries a store key, `ref`. A folder name never
decides visibility, and no node can expose anything: the owner does, per
folder, in Studio → Files. Never put `$trigger.files` or base64 into a payload, a script
return, or a database column.

## Rich text: the editor

`import { Editor } from "zeb/ui/editor"` — a Notion-style block editor whose
value is a ProseMirror document (JSON).

```tsx
import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml, documentText } from "zeb/ui/editor-render";

const [doc, setDoc] = useState(input.query?.rows?.[0]?.body_json ?? null);

async function uploadImage(file) {                       // the page decides where images go
  const form = new FormData();
  form.append("file", file);
  const { ref } = await (await fetch("/api/upload", { method: "POST", body: form })).json();
  return { src: `/_files/${ref}`, ref, alt: file.name };   // a folder the owner exposed public_read
}

<Editor value={doc} onChange={setDoc} uploadImage={uploadImage} placeholder="Write…" />
```

- **Store `body_json`** (the document). Derive `body_html = renderDocumentHtml(doc)`
  at save time for feeds, e-mails and search, and `excerpt = documentText(doc).slice(0, 200)`.
  Never store only the HTML — it cannot be edited faithfully again.
- **Render** a stored document with `<DocumentView doc={row.body_json} />` on
  the public page: server-rendered, no editor code shipped.
- **Images** go through `uploadImage`, which the page wires to a real
  upload pipeline (above). Without it the image block is not offered.
- **Saving** is a normal POST pipeline: the page submits `JSON.stringify(doc)`
  (a hidden input or a `fetch`), the pipeline validates it is an object with
  `type: "doc"`, stores `body_json`, `body_html`, `excerpt`.
- **Legacy HTML** (a previous CMS) converts once with `htmlToDocument(html)`
  from `zeb/prosemirror`, in a migration job, then lives as JSON.

## Prove it

1. Upload a real file through the form; `pipeline_get_invocations` shows
   `fs.file.put` and the `ref` it wrote; the file loads where the page points
   (`curl -I` → 200, right `Content-Type`); the private form returns 401
   without a session.
2. An upload over `--max-size` or of the wrong kind is refused with a message
   the form shows.
3. In the editor, type `/`, pick Image, choose a file: it appears in the
   editor and in the `DocumentView` beside it, and the stored `body_json`
   contains an `image` node with `ref`.
4. Save, reload the page: the editor shows the same document; the public
   page renders it without a console error.
