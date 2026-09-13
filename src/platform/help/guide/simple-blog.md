# Simple Blog Project

A simple blog is a good baseline Zebflow app because it uses both pipelines and templates clearly.

## Typical pieces

- public home page
- post detail page
- admin page
- create/edit/delete flows
- data storage in Sekejap or PostgreSQL

## Example structure

A project's source root is the repository root (unless `zebflow.yaml` sets
`spec.layout.source`) — there is no separate `repo/pipelines/` or
`repo/templates/` tree. Pipelines, pages, and components all live at the root:

```text
api/
└── posts.zf.json          register api/posts -- | trigger.webhook …
pages/
├── home.zf.json           register pages/home -- | trigger.webhook … | web.response --template pages/home.tsx
├── post-detail.zf.json
├── admin.zf.json
├── home.tsx
├── post-detail.tsx
└── admin.tsx
components/
docs/
```

A page is two things wired together: a pipeline (`pages/home.zf.json`) whose
`web.response` node points `--template` at the matching `.tsx` file
(`pages/home.tsx`). Data pipelines under `api/` answer JSON instead.

## Why this example matters

It shows the normal Zebflow shape:

- route/page behavior in pipelines
- render logic in templates
- shared components and styles nearby
- project docs kept with the app

See `help("pipeline/examples/blog-with-admin")` for a fuller example recipe.
