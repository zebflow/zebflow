# Project layout — the contract as it exists today

Status: **survey**, amended after the resolver landed. Nothing here is a
proposal. It exists so that a declared layout can be written without missing a
consumer.

Section 5 records what changed. Sections 1 to 4 describe the rules as they were
found, and the line numbers in them are the ones the survey was taken against;
they have moved.

Every rule below is still decided by the platform rather than declared by the
project, which is why no project can use a folder pattern of its own.

## 0. The fact that explains the rest

`repo/pipelines/` is not the pipelines directory. It is **the source tree**.

`layout.repo_pipelines_dir` is the RWE template root: pages, stylesheets and
shared components compile from it (`ops.rs:1674`, `ops.rs:1951`,
`node_registry.rs:1571`). A real application looks like this:

```
repo/
  pipelines/
    feed.zf.json            ← a pipeline
    pages/spatial-blog/*.tsx ← pages, compiled by RWE
    styles/main.css          ← stylesheets
    shared/ui/*.tsx          ← components
  docs/
  schemas/
  seeds/sekejap/*.sql
  zeb.lock
  zebflow.yaml
```

So "move to one source root" is mostly a **rename plus pulling three strays in**,
not a new architecture. The one tree already exists and is misnamed.

## 1. Three concerns are tangled, and they are not the same problem

Discussions of layout treat this as one question. The code has three, and they
fail differently.

### Identity — what names a pipeline

`normalize_pipeline_file_rel_path` (`project.rs:2879`) forces any path to start
with `pipelines/`, and appends `.zf.json` only when the path ends with neither
`.zf.json` nor `.json`. A path ending `.json` is left alone — so this function
can mint an identity that the `ends_with(".zf.json")` discovery predicate then
rejects. Identity and discovery already disagree inside the rule itself. It has **10 callers** across
`project.rs`, `ops.rs` and `mcp/handler.rs`.

This is not a read. It rewrites. `file_rel_path` is the sole persisted
identifier for a pipeline, so this function defines pipeline identity, and that
identity is stored in the database.

**Consequence for any layout change: a project that declares a different root
invalidates every persisted `file_rel_path`.** That is a migration, not a
config switch. This is the single hardest constraint in the plan.

It also means some callers depend on being lied to: they hand in a bare name and
rely on the prefix being added. Those call sites must be found before the rule
can consult a declaration.

### Discovery — which files are of which kind

| Kind | Rule | Where |
| --- | --- | --- |
| Pipeline | `starts_with("pipelines/") && ends_with(".zf.json")` | 44 sites, incl. `policy/package.rs:258`, `hub.rs:2348/2474/2539/4804/5056`, `ops.rs:1722`, `bin/zebflow.rs:605` |
| Page / style / component | anything under `repo_pipelines_dir`, resolved by RWE | `ops.rs:1674`, `ops.rs:1951` |
| Doc | `repo_dir.join("docs")` | `adapters/file/mod.rs:85`, `project.rs:2115` |
| Seed / initial data | six fixed prefixes in `INITIAL_DATA_DIRS` | `policy/package.rs` |
| Schema | `repo_schema_dir(...)/schema.json` | `sekejap.rs:866` |
| Node interface | `repo/nodes` | `repo_node_interfaces_dir` |
| Asset / image | `repo/pipelines/assets/`, created for every project | `adapters/file/mod.rs:112`, served by `web/mod.rs:2069` and `:2153` |

The pipeline rule was duplicated rather than shared. Three of the four copies
agreed. `ops.rs:1722` did not: it is `ends_with(".zf.json") || starts_with("pipelines/")`,
an **or** over a different namespace. It decides whether `move_resource` is
moving a pipeline, whose paths are repository-relative, or a template, whose
paths are relative to the source root. Read as the discovery predicate it looks
like a bug; read as the question it actually asks, the `or` is correct and the
table above was wrong to list it as the same rule.

Images do have a home: `ensure_project_layout` creates `repo/pipelines/assets/`
alongside `styles/` for every project, and two routes serve it. An earlier draft
of this document said assets were undefined. That was wrong.

### Placement — where an install writes

`install_root_for_target_folder` and `normalize_install_target_folder`
(`hub.rs:4467`, `hub.rs:4505`) decide the destination, and re-prefix a folder
with `pipelines/` only when the user typed a leading slash.

Observed live, same package, three target folders:

| `target_folder` | `install_root` | reviewed | registered |
| --- | --- | --- | --- |
| `""` | `pipelines/hub/{id}` | yes | yes |
| `billing` | `billing` | **no** | **no** |
| `/billing` | `pipelines/billing` | yes | yes |

The leading slash increases safety, which is inverted from any user's
expectation. Review and registration agree with each other — they share the
predicate — so nothing unreviewed executes. The failure is that the package
installs, the review reports `risk_level: low` with every finding list empty,
and the pipelines silently never run.

## 2. What a declaration has to satisfy

Derived from the above, not invented:

1. One resolver, because duplicated rules drift.
2. Identity must survive, or migrate deliberately. Persisted `file_rel_path`
   values encode `pipelines/`.
3. The RWE template root and the pipeline discovery root are the same directory
   today; a declaration must either keep that true or teach RWE otherwise.
4. Placement must follow the declaration, so that installing into any folder
   keeps discovery, review and registration aligned by construction.
5. Assets need a kind at all.
6. Callers relying on the rewrite adding a prefix must be found first.

## 3. Files that carry meaning by name, not location

These are matched by exact filename anywhere they are looked for, and are not
part of the kind-by-extension scheme:

- `zebflow.yaml` — project configuration; an install refuses a bundle without it
- `zeb.lock` — dependency lock
- `zebflow.init.json` — initialization payload
- `schema.json` — under the schema directory
- `AGENTS.md`, `SOUL.md` — assistant instruction files. These live in
  `data/runtime/agent_docs`, NOT in `repo/`, and are not git-synced. Note that
  `docs/contracts/project.md` says they belong in `repo/`; the code disagrees,
  and the code is what runs.

## 4. Extensions currently present in a repo

`.tsx` `.ts` `.css` `.zf.json` `.md` `.sql` `.json`

No repo in this codebase currently contains an image, though `pipelines/assets/`
exists and is served for exactly that purpose. Any allowlist should be derived
from this set plus a decided asset set, rather than from what seems reasonable.

## 5. What the resolver changed

`ResolvedProjectLayout` (`platform/model.rs`) now answers every question in
section 1 that is not identity. `ProjectFileLayout` carries one and derives
`repo_source_dir()`, `repo_assets_dir()`, `repo_docs_dir()` and
`repo_node_interfaces_dir()` from it, so an absolute path and the
repository-relative rule that names it cannot disagree. The safety review, the
prepared-install gate, and every place that decides which installed files to
register now share `is_pipeline_rel_path`; placement shares `source_rel` and
`is_in_source` with them. The installer's initial-data table is reached through
`initial_data`, and `INITIAL_DATA_DIRS` is what the default is derived *from*
rather than a second reader.

Nothing became free-form. `FilesystemFileAdapter::ensure_project_layout` builds
the resolver from `ResolvedProjectLayout::platform_default()` and never opens
`zebflow.yaml`, so every project resolves to the directories in section 0. That
one line is the seam a declared layout arrives through.

Identity is untouched, exactly as section 1 warns. `normalize_pipeline_file_rel_path`,
`virtual_path_from_file_rel_path` and `name_from_file_rel_path` still hardcode
`pipelines/`, because a persisted `file_rel_path` encodes it and changing that
is a migration.

Two things section 1 called settled were not. `ops.rs:1722` is an `or`, as
noted above. And the runtime snapshot path stripped its prefix with
`trim_start_matches`, which repeats, where every other copy used
`strip_prefix`, which does not; it now strips once like the rest. Nothing
reachable can produce the doubled prefix that told them apart.
