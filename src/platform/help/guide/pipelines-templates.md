# Pipelines + Templates

Pipelines and templates are the two core building blocks of Zebflow apps.

## Pipeline

A pipeline is the runtime graph:

- what triggers the flow
- what nodes run
- what data moves through the graph
- what response goes back out

## Template

A template is the UI/runtime surface:

- TSX page structure
- components
- browser interactivity
- Zeb React hooks
- `zeb/*` frontend libraries

## How they combine

The usual pattern is:

1. a webhook trigger receives the request
2. data nodes fetch or shape data
3. a `n.web.response` node renders a TSX page

That means Zebflow does not treat “frontend” and “backend” as separate products.
They are two parts of one project graph.

## Mental shortcut

- pipeline = behavior
- template = experience

Most real apps in Zebflow are the composition of both.
