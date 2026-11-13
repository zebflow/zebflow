# zeb/markdown

```tsx
import { Markdown, renderMarkdown } from "zeb/markdown";
```

`zeb/markdown` is the bundled markdown rendering surface for Zebflow pages.

Use it when you need:

- documentation rendering
- article bodies
- notes
- generated help content

It should be the preferred markdown renderer inside Zebflow web UI instead of ad hoc rendering logic.

See `help("web/libraries")` for the API.

## Styling

```tsx
<Markdown content={post.body} className="prose max-w-none" />
```

- `prose` styles what the Markdown renders: headings, lists, links, code,
  quotes and tables, drawn with the theme's colours. It works in any project;
  no stylesheet of your own is needed.
- `prose-sm` and `prose-lg` change the size; `max-w-none` lets it fill the
  width.
- `className` is the name every zeb/ui component takes; `class` works too.
