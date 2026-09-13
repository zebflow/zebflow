---
name: zebflow-engineering
description: How a Zebflow project is organised so it stays consistent as it grows — where a pipeline, page, component, query or migration goes, when a thing becomes shared, one shell for the whole app, one shape per entity. Use before the first file of a new project, before adding a new domain or entity to an existing one, and whenever you are about to write something that might already exist.
license: MIT
metadata:
  version: "1"
---

# Engineering: shape over layout

A Zebflow project is a tree of files the platform reads by path. The platform
fixes almost nothing about that tree (`public/`, `docs/`, `skills/`,
`shared/ui/`, `initial-data/` — the rest is yours), so the tree is a decision
the project makes once and every session follows. This skill is that
decision's shape. Facts: `help(topic="platform/agent")`, `zebflow-rwe`,
`zebflow-pipeline`.

## First: read the project's own layout

1. `file_read rel_path="docs/structure.md"`. If it exists, it wins over this
   skill — follow it exactly, including its names.
2. If it does not exist, choose one of the two layouts below, write it to
   `docs/structure.md` in ten lines (which layout, the modules you foresee,
   where shared things go), and follow it. The next session reads that file,
   not your memory.

## Two default layouts

**Flat** — for a small project: one domain, under about ten routes, one
author. Kind folders at the root, nothing else:

```
api/        pipelines that answer JSON        api/posts/create
pages/      pipelines + templates that render pages/posts.tsx, pages/posts (pipeline)
jobs/       schedules and function pipelines  jobs/digest
components/ reusable pieces                   components/post-card.tsx
db/         migrations, numbered              db/001_posts.sql
docs/  shared/ui/  initial-data/              platform folders
```

**Domain** — for anything else, and the default when in doubt. The root holds
only `shared/` and `modules/`, plus the platform folders. Every module has
the same shape as the root:

```
shared/                          what two or more modules use
  ui/                            clone-to-own primitives (platform-owned)
  components/page-shell.tsx      the one shell every page renders inside
  components/…  lib/…            app-wide components, pure functions
db/                              app-wide migrations (a module may own its own db/)
modules/
  finance/
    api/  pages/  jobs/  components/  db/
    shared/                      only because invoicing and payroll both use it
    modules/
      invoicing/  { api/ pages/ components/ }
      payroll/    { api/ pages/ jobs/ }
  employee/
    api/  pages/  components/
```

`modules/` is not decoration: without it, `finance/invoicing/` could be a
submodule or a kind folder, and a reader has to guess. With it, the root and
every module are the same thing — `{ kind folders, shared/, modules/ }` —
so a tool, a skill or a new session can walk any depth without asking.

A pipeline's `file_rel_path` is its location in this tree
(`modules/finance/modules/invoicing/api/create`); its URL is whatever
`--path` says. A template is referenced by its full path
(`--template modules/finance/pages/invoices.tsx`, import
`@/modules/finance/components/invoice-row`). Depth costs nothing.

## The rules that hold under either layout

1. **A folder is a module, `shared/`, or a kind folder.** Nothing else. A
   module contains only the kind folders it needs (`api/ pages/ jobs/
   components/ db/`), an optional `shared/`, and an optional `modules/`.
2. **Lowest level that contains all users.** A component used by one page
   sits in that module's `components/`. Used by two sibling modules, it moves
   to their parent's `shared/`. Used across the app, root `shared/`. A
   `shared/` folder with one consumer is a mistake; a module with no
   submodules and no sibling sharing has no `shared/` at all.
3. **Written twice is in the wrong place.** Before writing a component, a
   query or a script, `file_search pattern="<its name or its key phrase>"`. If
   it exists, import it or lift it; do not write a second one. The same for
   pipelines: `pipeline_list` before a new route.
4. **One shell.** `shared/components/page-shell.tsx` renders header,
   navigation, width, footer and the toast region; every page returns
   `<PageShell …>` and nothing but its own content inside. A module may add
   a `module-shell.tsx` that renders inside `PageShell`. Consistency comes
   from the nesting, not from every page remembering the same classes.
5. **Theme roles only.** `bg-background text-foreground border-border
   bg-primary text-muted-foreground …` (`help(topic="web/tailwind")`). Never a
   palette colour (`bg-slate-800`), an arbitrary value (`w-[347px]`) or an
   inline style for colour or spacing. A page that needs a new role is a
   change to the theme, once, not a class on the page.
6. **One shape per entity.** Each table is documented once in
   `docs/schema.md` (columns, formats, enums); every query selects the
   columns a page needs by name; a page receives rows and renders them — it
   never decides which database to ask (`zebflow-data`).
7. **A page is done with its states.** The empty state ("no invoices yet",
   with the action that creates one), the error branch (`logic.if` → a
   `web.response --status 4xx` with a rendered message, not a bare JSON
   error), and for every form the route it lands on afterwards.
8. **Migrations are files, numbered, never edited after they ran.**
   `db/003_invoices_add_due.sql`, applied by `jobs/migrate` or a one-off
   `pipeline_run`; a schema change without a file did not happen.
9. **Names are boring and the same everywhere.** Folders and files
   `kebab-case`; a route and its template share a name
   (`pages/invoices` ↔ `pages/invoices.tsx`); a module is a noun
   (`invoicing`, not `invoice-stuff`); a component is what it renders
   (`invoice-row`, `empty-state`).

## Adding a domain or an entity

The test of a good tree is that the fifth entity is added the same way as the
first, in known places:

1. `docs/structure.md` — add the module (or confirm which one it belongs to).
2. `docs/schema.md` + `db/NNN_<entity>.sql` — the table, applied and read back
   with `connection_describe`.
3. `modules/<m>/api/<entity>/{list,create,update,delete}` — JSON routes,
   validated `input.body`, the same status codes as the other modules.
4. `modules/<m>/pages/<entities>.tsx` + `<entity>.tsx` — list and detail
   inside `PageShell`, using `shared/components` first, module components
   second, a new component last.
5. `modules/<m>/components/` — only what steps 3–4 needed twice.
6. `MEMORY.md` — one line: what was added, where, what is open.

If step 3 or 4 touched more than about six files, the tree is fighting you:
stop and fix the shape (usually a missing `shared/` or a module that should
be two) before adding more.

## Prove it

- `file_list` of the module shows only kind folders, `shared/`, `modules/`.
- `file_search pattern="PageShell"` matches every page; a page without it is
  a defect.
- `file_search pattern="bg-slate"` (and `text-gray`, `#`, `style={{`) finds
  nothing under `pages/` or `components/`.
- Two pages of the same kind (two lists, two forms) in different modules
  look like the same application. If they do not, the shared component is
  missing, not the polish.
