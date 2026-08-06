# Mounted Data Root

## Definition

`{data_root}` is the real mounted persistence root for one Zebflow office.

In Docker or Kubernetes this is the mounted volume path, for example:

```text
/var/lib/zebflow/data
```

Do not create a nested `mounted/` directory inside `{data_root}`. Every durable
Zebflow folder lives directly below `{data_root}`.

## Top-Level Contract

```text
{data_root}/
  platform/
  users/
  services/
  libraries/
```

Top-level responsibilities:

| Path | Owner | Purpose |
| --- | --- | --- |
| `platform/` | platform control plane | Catalog DB, controller metadata, operation journals, platform caches |
| `users/` | project system | User/project workspaces, project source, project runtime data, project files |
| `services/` | platform services | Office-hosted service bodies such as hub |
| `libraries/` | library system | Installed and available Zeb libraries, indexes, compatibility metadata, and rebuildable library caches |

## Full Hierarchy

```text
{data_root}/
  platform/
    catalog.db
    operations/
      project/
      service/
    cache/

  users/
    {owner}/
      {project}/
        repo/
          .git/
          zebflow.json
          pipelines/
          docs/
        data/
          local.db
          sekejap/
          runtime/
            pipelines/
            agent_docs/
            web-assets/
              rwe/
                chunks/
        files/
          ...

  services/
    hub-default/
      service.json
      hub.db
      packages/
        {package_id}/
          package.json
          versions/
            {version}/
              manifest.json
              artifact.json
              readme.md
              media/
                {asset_id}.{ext}
      publishers/
        {publisher_id}/
          publisher.json
      tokens/
      audit/
        hub-events.jsonl
      cache/

  libraries/
    catalog.db
    installed/
      zeb/
        preact/
          0.1/
        threejs/
          0.1/
        deckgl/
          0.1/
      external/
        js-registry/
          {package}/
            {version}/
              package/
    indexes/
      external/
        js-registry/
          {package}/
            {version}/
              exports.json
    downloads/
      external/
        js-registry/
    cache/
      external/
        js-registry/
          tmp/
```

## Invariants

- `{data_root}` is the mount. There is no `{data_root}/mounted/` in the storage
  contract.
- `platform/` stores control-plane metadata, not project business payloads.
- `users/{owner}/{project}/` stores project source, runtime state, and files.
- `services/{service_instance_id}/` stores service-owned runtime state and
  artifacts.
- Hub service storage must not include owner or project slugs in its
  filesystem paths.
- Rebuildable caches belong inside the owning domain, for example
  `platform/cache/`, `services/{service_instance_id}/cache/`, or
  `libraries/cache/`.
- There is no top-level `tmp/` in the formal contract. If temporary workspace is
  needed later, it belongs inside the domain that owns the operation.

## Libraries Root

The libraries root is:

```text
{data_root}/libraries/
```

Libraries are versioned independently from the Zebflow binary. A Zebflow update
may make new library versions available, but existing projects keep their pinned
library versions until the user explicitly upgrades them.

```text
{data_root}/libraries/
  catalog.db
  installed/
    {namespace}/
      {library}/
        {version}/
          library.json
          exports.json
          runtime/
          wrappers/
  indexes/
    {namespace}/{library}/{version}/
  downloads/
  cache/
```

The current external package adapter stores fetched package archives and
extracted package bodies under the library domain:

```text
{data_root}/libraries/
  installed/external/js-registry/{package}/{version}/package/
  indexes/external/js-registry/{package}/{version}/exports.json
  downloads/external/js-registry/
  cache/external/js-registry/tmp/
```

Project pins live in project configuration, not in `libraries/`:

```text
users/{owner}/{project}/repo/zebflow.json
```

## Hub Service Root

The default hub service root is:

```text
{data_root}/services/hub-default/
```

Package version artifacts use:

```text
services/hub-default/packages/{package_id}/versions/{version}/artifact.json
```

This path is stored relative to `{data_root}`.

## Current Migration Note

Older development builds may have created a nested legacy package cache under
the data root. That shape is outside the formal mounted data-root contract and
is no longer a valid writer path. Fresh installs and current library preparation
write under:

```text
{data_root}/libraries/
```
