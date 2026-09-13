# zeb/ui — components for project pages

shadcn/ui's components, on Zebflow's engine. Nothing to install: a page
imports one file per component and the compiler inlines it.

```tsx
import { Button } from "zeb/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "zeb/ui/card";
import "@/globals.css";

export default function Page() {
  return (
    <Card>
      <CardHeader><CardTitle>Members</CardTitle></CardHeader>
      <CardContent>
        <Button variant="outline" size="sm">Add member</Button>
      </CardContent>
    </Card>
  );
}
```

The same names, variants and sub-parts as shadcn: `variant="default | secondary | outline | ghost | destructive | link"`, `size="default | xs | sm | lg | icon | icon-xs | icon-sm | icon-lg"`, `DialogContent`, `DialogHeader`, `TabsTrigger`… A snippet written for shadcn reads the same here.

## What is different from shadcn

- **No `asChild`.** Pass `as="a"` and an `href` to render a button as a link.
- **Composition works as in Radix**: `<Tabs value defaultValue onValueChange>` and
  `<Dialog open defaultOpen onOpenChange>` share state with their sub-parts
  through context, so `<DialogTrigger>` opens the dialog and `<TabsTrigger value="a">`
  selects its tab without wiring. Each component's header comment says exactly
  what it takes; `/dev/design-system/ui` shows every one working.
- **Variants are plain objects joined by `cx`**, not `cva`. Every class string is a literal.
- **Colours are theme roles.** `bg-primary`, `text-muted-foreground`, `border-input`.
  The project's `globals.css` defines every role for light (`:root`) and dark
  (`.dark`); put `dark` on any ancestor to switch. A theme exported from
  ui.shadcn.com or tweakcn replaces the two blocks — it must give every
  token a complete colour value (`#…`, `hsl(…)`, `oklch(…)`), and keep the
  three extra pairs `success`, `warning`, `info`.

See every component with its variants at `/dev/design-system/ui`; the theme
tokens themselves are at `/dev/design-system`.

## The editor

`zeb/ui/editor` is a Notion-style block editor: type `/` for blocks, select
text for the toolbar, Markdown shortcuts (`# `, `- `, `[] `, `> `, ```` ``` ````,
`---`), hover a block for its drag handle, paste or drop an image. Its value
is a document (JSON) — store that, and render it anywhere with
`zeb/ui/editor-render`.

```tsx
import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml, documentText } from "zeb/ui/editor-render";

const [doc, setDoc] = useState(post.body_json);

// Images go wherever the page sends them — here a webhook running n.fs.save.
async function uploadImage(file) {
  const form = new FormData();
  form.append("file", file);
  const { saved } = await (await fetch("/wh/o/p/upload", { method: "POST", body: form })).json();
  return { src: `/files/o/p/${saved.path}`, ref: saved.path, alt: file.name };
}

<Editor value={doc} onChange={setDoc} uploadImage={uploadImage} placeholder="Write…" />
<DocumentView doc={doc} />                  // elements, on the server too
renderDocumentHtml(doc)                     // the HTML string for a body_html column
documentText(doc).slice(0, 160)             // a teaser
```

Without `uploadImage` the image block is not offered. `readOnly` shows the
document without editing. The engine underneath is `zeb/prosemirror`
(`help(topic="web/libraries")`) — the one runtime dependency in zeb/ui,
loaded only on pages that import the editor. Code blocks are coloured by the
same tokenizer as `zeb/ui/code-block`.

## Owning a component

To change one, clone it into the project:

```
install_ui_components names=["dialog"]        # MCP tool; list_ui_catalog shows what exists and what is cloned
```

It lands at `shared/ui/dialog.tsx`, yours to edit. Its `zeb/ui/*` imports
still resolve to the library, so it works unchanged; switch the one import
in your page to `@/shared/ui/dialog`. Everything you did not clone keeps
coming from `zeb/ui`.

## Where the files are

`blessed/source-libraries/ui/<version>/src/<name>.tsx` in the platform, one
family per file, named as shadcn names them. `zeb/ui/hooks` holds the shared
behaviour (`useClickAway`, `useEscape`, `useFocusTrap`, `useControllable`,
`useAnchoredPosition`) and may be imported by a page too.
