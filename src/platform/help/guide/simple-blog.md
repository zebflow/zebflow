# Simple Blog Project

A simple blog is a good baseline Zebflow app because it uses both pipelines and templates clearly.

## Typical pieces

- public home page
- post detail page
- admin page
- create/edit/delete flows
- data storage in Sekejap or PostgreSQL

## Example structure

```text
repo/
├── pipelines/
│   ├── pages/
│   │   ├── home.zf.json
│   │   ├── post-detail.zf.json
│   │   └── admin.zf.json
│   └── api/
│       └── posts.zf.json
├── templates/
│   ├── pages/
│   │   ├── blog/
│   │   │   ├── home.tsx
│   │   │   ├── post.tsx
│   │   │   └── admin.tsx
│   ├── components/
│   └── styles/
└── docs/
```

## Why this example matters

It shows the normal Zebflow shape:

- route/page behavior in pipelines
- render logic in templates
- shared components and styles nearby
- project docs kept with the app

See `help("pipeline/examples/blog-with-admin")` for a fuller example recipe.
