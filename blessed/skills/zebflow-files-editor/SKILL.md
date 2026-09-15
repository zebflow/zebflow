---
name: zebflow-files-editor
description: Uploads, images, files and rich text in a Zebflow project — FileRef, fs.save and fs.thumbnail, public vs private URLs, the zeb/ui editor and how its document is stored and rendered. Use before building an upload form, an image field, a media library, or any page with authored rich content.
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
2. The webhook delivers the file as `input.files.photo` — a FileRef
   (`ref`, `filename`, `mime`, `kind`, `size`, `sha256`, `lifecycle: temporary`).
   It is discarded after the run unless a node keeps it.
3. Keep it: `fs.save --field photo --folder public/uploads --allowed-kinds images --max-size 10`
   adds `saved: { path, url, original_name, content_type, size }` to the
   payload; `input.body.caption` from the same form is still there.
4. Derive what you need: `fs.thumbnail --width 320 --height 320 --fit cover --format webp --folder public/thumbs --source-key saved.path`
   adds `thumbnail` (a FileRef, `thumbnail.ref`) the same way.
5. Store the **path** (`saved.path`) in your table, not a URL — URLs depend on
   owner, project and visibility.

```
| trigger.webhook --path /api/upload --method POST --auth-type jwt --auth-credential jwt_main
| fs.save --field file --folder public/uploads --allowed-kinds images --max-size 10
```

The response carries `saved: { path, … }`. Store the **path**; a page
writes the URL as a root-relative path on the project's own host and the
renderer makes it absolute (`docs/contracts/addressing.md`).

## Where a file is reachable

| Stored under | Path a page writes | Who |
|---|---|---|
| `public/…` | `/_files/…` (the part after `public/`) — e.g. `public/photos/jane.webp` → `/_files/photos/jane.webp` | anyone; the only kind an `og:image` or an `<img>` on a public page may use |
| anything else | `/_fs/…` | a signed-in session with files access |
| either, from outside a page (a tool, a mail) | the platform form `/files/{owner}/{project}/public/…` · `/fs/{owner}/{project}/…` — valid on every host, but it carries owner and project, so not in pages |

`saved.url` is the private platform form. Decide visibility by folder when
you save, not afterwards. Never put `input.files` or base64 into a payload, a script
return, or a database column.

## Rich text: the editor

`import { Editor } from "zeb/ui/editor"` — a Notion-style block editor whose
value is a ProseMirror document (JSON).

```tsx
import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml, documentText } from "zeb/ui/editor-render";

const [doc, setDoc] = useState(input.rows?.[0]?.body_json ?? null);

async function uploadImage(file) {                       // the page decides where images go
  const form = new FormData();
  form.append("file", file);
  const { saved } = await (await fetch(`${input.base}/api/upload`, { method: "POST", body: form })).json();
  return { src: `${input.files}/${saved.path}`, ref: saved.path, alt: file.name };
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
   `fs.save` with the path; the URL loads (`curl -I` → 200, right `Content-Type`);
   the private form returns 401 without a session.
2. An upload over `--max-size` or of the wrong kind is refused with a message
   the form shows.
3. In the editor, type `/`, pick Image, choose a file: it appears in the
   editor and in the `DocumentView` beside it, and the stored `body_json`
   contains an `image` node with `ref`.
4. Save, reload the page: the editor shows the same document; the public
   page renders it without a console error.
