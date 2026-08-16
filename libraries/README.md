# Zeb Libraries

This folder contains the browser libraries that Zebflow reviews, bundles, and
ships with the platform.

## Current Model

Zebflow embeds these files in the executable and serves them from local asset
routes. A project enables a library in Project Settings. Zebflow records the
requested state in `repo/zebflow.yaml` and the exact resolved artifact in
`repo/zeb.lock`.

The project does not run `npm`, create `node_modules`, or download a package
when a template is saved.

## Supported Dependency Paths

1. Write a focused TypeScript module in the project under `repo/pipelines/`.
2. Add a reviewed script or template package from Zebflow Hub.
3. Enable a bundled `zeb/*` library and pin it through `zeb.lock`.

Direct npm ingestion is not a supported project feature. It requires a complete
supply chain policy for transitive dependencies, lifecycle scripts, archive
limits, compilation, integrity, and capability review. That work remains a
future direction.

## Maintainer Rule

Upstream npm tools may be used outside Zebflow to build an official bundled
library. The reviewed output committed here is the runtime artifact. End-user
projects never depend on that build toolchain.
