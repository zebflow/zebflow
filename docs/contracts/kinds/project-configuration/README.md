# ProjectConfiguration

Status: **Frozen** on 2026-08-15. Amended 2026-08-20 to add optional
`spec.layout` and, later the same day, `spec.layout.allowed_extensions`, and on
2026-09-01 to add optional `spec.files.backend`; see
[Version Rules](#version-rules).

`ProjectConfiguration` is the portable, non-secret project configuration stored
in `repo/zebflow.yaml`. It is tracked with project source and remains useful when
the repository moves to another Zebflow instance.

This is the reference implementation for every other Zebflow contract kind. A
kind is not frozen until it has the same level of identity, ownership,
validation, migration, recovery, fixtures, and documentation.

## Identity

| Item | Value |
| --- | --- |
| API version | `zebflow.com/v1` |
| Kind | `ProjectConfiguration` |
| Canonical file | `repo/zebflow.yaml` |
| Owner | The project named by the owning path |
| Rust definition | `src/contracts/kinds/project_configuration.rs` |
| Read and write service | `src/platform/services/project_config.rs` |
| Golden fixture | `tests/fixtures/contracts/project-configuration/v1-complete.yaml` |
| Declared layout fixture | `tests/fixtures/contracts/project-configuration/v1-layout.yaml` |

`metadata.name` must be the canonical project slug and must match the project
slug in the owning path. `metadata.version`, `metadata.digest`, and
`metadata.annotations` are forbidden. This mutable project document has no
independent package release identity.

## Canonical Shape

The canonical representation is strict, UTF-8 YAML using only the
JSON-compatible subset:

```yaml
apiVersion: zebflow.com/v1
kind: ProjectConfiguration
metadata:
  name: project-development
spec:
  profile:
    title: Project Development
  layout:
    source: src
    initial_data:
    - path: db/seeds
      engine: sekejap
    allowed_extensions:
    - tsx
    - ts
    - css
    - json
    - md
  rwe: {}
  pipelines:
    logging:
      max_invocations: 25
    node_timeout_secs: 120
  runtime:
    mode: dedicated
    execution: resident
    resource_profile: small
  bootstrap: {}
  git:
    remote:
      credential_id: git-origin
      repo_url: https://git.example.com/team/project.git
      branch: main
  assistant: {}
  locks: {}
  data: {}
  files:
    backend: zebfs
    uploads:
      max_asset_size_mb: 50
      webhook_body_max_mb: 512
      max_file_size_mb: 512
  distribution:
    hub: {}
```

The full canonical example is the golden fixture linked above.

## Field Ownership

| Field | Owns | Must not contain |
| --- | --- | --- |
| `metadata.name` | Project identity | A display title or release version |
| `spec.profile` | Display title and description | Authentication or placement state |
| `spec.layout` | Repository-relative directories the project's own files live in, and the file extensions it accepts | Absolute paths, platform data directories, or file contents |
| `spec.rwe` | RWE policy and requested libraries | Resolved library files or integrity hashes |
| `spec.pipelines` | Pipeline logging retention and node timeout | Pipeline definitions or invocation records |
| `spec.runtime` | Portable execution and resource intent | Worker IDs, pod names, or live allocation state |
| `spec.bootstrap` | Project-relative activation paths | Runtime snapshots |
| `spec.git` | Remote URL, branch, and credential reference | Git passwords, tokens, or private keys |
| `spec.assistant` | Credential references and bounded behavior | API keys or chat history |
| `spec.locks` | Project-relative protected template paths | Access-control policy or file contents |
| `spec.data` | Reserved future data policy | Records, schema, or connection secrets |
| `spec.files` | The native store this project's files live in, and upload size policy | Uploaded bytes, object metadata, or an endpoint, bucket, or credential |
| `spec.distribution` | Hub publishing intent | Hub access tokens or published package content |

Credential and connection IDs are references. Their secret values remain in the
project credential store.

`spec.rwe.libraries` is the requested project state. `zeb.lock` owns resolved
entry paths and integrity hashes. They may repeat a requested version and source
for verification, but they do not own the same decision.

### spec.layout

Every entry is one directory, relative to `repo/`, and every entry is optional.
An absent entry means the default in [Frozen Defaults](#frozen-defaults).

**A layout entry names where a kind is looked for. It is never a directory the
platform creates.** A project starts as an empty repository; a folder exists
because an author made one, and every writer creates its own parents. Scaffolding
them ran on every request, so a folder the author deleted came back on the next
page load.

`source` is one directory, not two: it is the root that holds pipelines, pages,
stylesheets, and shared components, and it is the same directory the RWE
compiler treats as its template root and `@/` import root. **It defaults to the
repository itself** — an empty declaration names the repository root — so a
pipeline, a page and a README sit side by side at the top until a project says
otherwise.

`static` is the one entry that may **not** be empty. Everything beneath it is
served by `GET /static/{owner}/{project}/…` with no authentication, so rooting it
at the repository would publish `zebflow.yaml`, `zeb.lock` and every pipeline
definition. It defaults to `static` inside whatever `source` resolves to, so a
project that moves its source does not leave a public directory behind in the
tree it moved out of.

`schema` and `sqlite_schema` are separate entries because they are different
documents written by different engines, and a project may carry one without the
other.

`initial_data` is an ordered list of prefixes an install would replay, each
bound to the database engine that would replay it. Declaring the list replaces
the default list rather than extending it, so a project that names one prefix
has exactly one.

`allowed_extensions` is the one entry that is not a directory. It is the set of
file extensions, lowercase and without a dot, that a package may write into
`repo/`; the Hub safety review turns a path outside it into a violation, and a
violation is never overridable, so both install gates refuse the package. It
sits with the directories because it answers the same question they do -- what
this project's repository is allowed to contain -- and every consumer that has
to agree on that already resolves through the same layout.

Declaring the list **narrows** the platform set in [Frozen
Defaults](#frozen-defaults). It cannot widen it: an entry outside the platform
set is refused by name when the document is read, rather than accepted or
dropped. A refusal a project could switch off in its own configuration would be
a warning wearing a violation's name, and the substring-matched effects the
review already reports are what warnings are for. If a project genuinely needs
a file type that is not here, that is evidence for amending the platform set --
one decision, visible to everyone -- not for a per-project escape hatch.

Some files are accepted by name whatever the extension list says:
`zebflow.yaml`, `zeb.lock`, `zebflow.init.json`, `schema.json`, and `.gitkeep`.
The first three travel in every install scope and an install refuses a bundle
without a `zebflow.yaml`; `.gitkeep` is written into three directories of every
project by `ensure_project_layout`. A project cannot narrow this list away,
because narrowing it is how a project would make its own exports uninstallable.

Discovery, placement, and the directories inside `repo/` are all resolved
through one resolver rather than repeated as literals, and the declaration is
what that resolver is given: `ensure_project_layout` reads this file, so a
project that declares a different `source` genuinely registers, compiles,
serves, and installs from it.

Pipeline identity is not a directory and is not declared here. A pipeline's
`file_rel_path` is its path *inside* `source`, so changing `source` does not
rename any pipeline. Ids persisted before that was true still carry the old
root; they are read through the same rule, and
`zebflow project pipelines migrate` rewrites the stored bytes once. See
[layout.md](./layout.md) for the survey these rules came from.

### spec.files

`backend` names the **native** store: where this project's own `files/` live,
what a `/fs/` read serves from, and the word every FileRef written here carries
in its `backend` field. The two must agree, because they name the same thing;
[file-ref](../file-ref/README.md) is the format half of the same decision.

It is not a pipeline's connection to somebody else's bucket. An external S3,
MinIO, or SeaweedFS a pipeline reads and writes is a *connection* with a
credential, the same shape as a Postgres connection and chosen per pipeline. A
node that reads from one produces bytes that land in the native store, so the
FileRef it emits carries the **native** backend word -- never `s3` on account of
where the bytes came from. Otherwise `ref` would stop meaning one thing:
sometimes a key in the store Zebflow owns, sometimes a key in a bucket it does
not. Both questions can be answered "S3" and still be different questions.

`zebfs` is the only accepted value in this build. It is the word the FileRef
field already carries for locally-stored bytes, so one constant spells it in
both places (`src/zebfs/backend.rs`). An unknown value is refused by name with
the accepted list rather than resolved to the default, because a project whose
bytes went to a store it did not declare is worse than a project that will not
start. A second backend is one more accepted value plus the connection that
holds its endpoint and credential; nothing about this section reshapes.

Declaring it is per project, because `files/` is per project and a store is what
a project's own objects are addressed in. An instance-wide bucket with
per-project prefixes is not excluded by that: it would be declared here as the
same word by every project on the instance, with the endpoint and prefix rule
living on the connection rather than repeated in each `zebflow.yaml`.

## Integration Map

Every v1 section has one writer path and one runtime owner. Settings controls
write through `ProjectConfigurationService`; they never edit YAML with direct
file operations.

| Section | User-facing writer | Runtime reader or effect |
| --- | --- | --- |
| `spec.profile` | Project creation and Settings > General | Project lists, headers, and app metadata |
| `spec.layout` | No writer; hand-authored in `repo/zebflow.yaml` | `FilesystemFileAdapter::ensure_project_layout` reads it into `ResolvedProjectLayout`, which every consumer resolves through: pipeline discovery and registration, the RWE template and `@/` root, asset serving, docs generation, initial-data replay, Hub install placement, and the Hub safety review's file-type refusal |
| `spec.rwe` | Settings > Policy and Settings > Libraries | RWE compilation, rendering, assets, and editor libraries |
| `spec.pipelines` | Settings > General and Settings > Logs | Node timeout and bounded invocation retention |
| `spec.runtime` | Project creation and explicit project configuration edits | Runtime synchronization and placement planning |
| `spec.bootstrap` | Project package installation | Pipeline activation during materialization and repo refresh |
| `spec.git` | Settings > General Git Remote | Git sync and push target selection |
| `spec.assistant` | Settings > Automatons | Project assistant credentials and execution bounds |
| `spec.locks` | Template editor lock control | REST, MCP, and assistant template access checks |
| `spec.data` | Reserved; no writer in v1 | No runtime effect in v1 |
| `spec.files` | Settings > General Runtime Defaults writes the upload limits; `backend` has no writer and is hand-authored in `repo/zebflow.yaml`, with Settings > Files showing which store is active | Asset, file, and webhook upload limits; `FilesystemFileAdapter::ensure_project_layout` reads `backend` into `ProjectFileLayout`, whose `open_files` is the one call that turns it into an implementation through `zebfs::backend::open` |
| `spec.distribution` | Settings > General presentation and Hub producer control | Dashboard app entry and Hub producer availability |

The complete `spec.runtime` section is preserved in project runtime bundles.
The current resident placement path applies `mode`, `resource_profile`, and
`min_replicas`. Non-resident `execution` modes, `max_replicas`,
`required_tags`, and `custom_resources` are portable intent for execution
backends that are not active yet; the current resident executor does not claim
to enforce them.

The Settings page also contains operational controls that intentionally do not
belong here. Credentials and Hub tokens are secrets in platform storage. Live
runtime placement, invocation rows, transfer history, Git health, caches,
database contents, and uploaded objects are environment or data state. Copying
them into `zebflow.yaml` would make the project unsafe to share and create two
sources of truth.

The internal `ZebflowJson` type is only a runtime view. It must map one-to-one
with this contract. It cannot contain extra writable fields that disappear when
the canonical YAML is written. Legacy-only fields belong to the explicit legacy
decoder and nowhere else.

## Frozen Defaults

Omitted fields use these v1 meanings:

| Field | Default |
| --- | --- |
| `spec.layout` | Every entry below; an absent section declares nothing |
| `spec.layout.source` | empty — the repository itself |
| `spec.layout.static` | `static` inside the resolved `source`, so `static` when `source` is undeclared. May not be empty |
| `spec.layout.docs` | `docs` |
| `spec.layout.schema` | `schemas/sekejap` |
| `spec.layout.sqlite_schema` | `schemas/sqlite` |
| `spec.layout.node_interfaces` | `nodes` |
| `spec.layout.initial_data` | `initial-data/sekejap`, `initial-data/sqlite`, `init/sekejap`, `init/sqlite`, `seeds/sekejap`, `seeds/sqlite`, each bound to the engine named in its own path |
| `spec.layout.allowed_extensions` | `css`, `geojson`, `js`, `json`, `jsx`, `md`, `mjs`, `sql`, `ts`, `tsx`, `txt`, `xml`, `yaml`, `yml`, `csv`, `gif`, `ico`, `jpeg`, `jpg`, `mp3`, `mp4`, `pdf`, `png`, `svg`, `ttf`, `webp`, `woff`, `woff2` |
| `spec.rwe.minify_html` | `false` |
| `spec.rwe.strict_mode` | `true` |
| `spec.pipelines.logging.max_invocations` | Runtime default of 20 |
| `spec.pipelines.node_timeout_secs` | Runtime default of 30 seconds |
| `spec.runtime.mode` | `shared` |
| `spec.runtime.execution` | `resident` |
| `spec.runtime.resource_profile` | `small` |
| `spec.runtime.min_replicas` | `1` |
| `spec.files.backend` | `zebfs` -- the native store on local disk |
| `spec.files.uploads.max_asset_size_mb` | Runtime default of 10 MiB |
| `spec.files.uploads.webhook_body_max_mb` | Runtime default of 100 MiB |
| `spec.files.uploads.max_file_size_mb` | Runtime default of 1024 MiB |

Changing any default meaning is a contract-breaking change. A writer omits
default values only when omission preserves these exact meanings.

## Validation

The reader rejects:

- unknown, missing, future-version, or wrong-kind root values
- unknown fields at every typed v1 section
- YAML anchors, aliases, explicit tags, duplicate keys, multiple documents, and
  non-string mapping keys
- files above 4 MiB, structures above 128 levels, or more than 100,000 parser
  events
- invalid project slugs, paths that escape the project, repeated paths or tags,
  malformed branches, and unsupported URLs
- layout directories that are empty, absolute, trailing-slashed, glob-bearing,
  or contain a `.` or `..` segment
- an empty `allowed_extensions` list, a repeated entry, and any entry outside
  the platform set -- including one spelled with a leading dot or in uppercase,
  because a declaration is matched, not normalized
- repeated initial-data prefixes, and initial-data engines other than `sekejap`
  and `sqlite`
- a `files.backend` other than `zebfs`, refused by name with the accepted list
- URLs with embedded credentials
- upload, timeout, logging, assistant, replica, or resource values outside their
  documented limits
- custom resources outside the custom profile, zero resource values, or requests
  above limits

Invalid API inputs are rejected. They are not clamped, guessed, or partially
saved.

## Read And Write Rules

| Boundary | Rule |
| --- | --- |
| Missing file | Use deterministic defaults in memory; the first write creates canonical YAML |
| Normal read | Accept only canonical v1 YAML |
| Normal write | Validate, write a same-directory temporary file, sync it, rename atomically, then sync the parent directory |
| Concurrent platform updates | Serialize read, modify, and write for one configuration path |
| Git | Track `zebflow.yaml` as project source |
| Hub export | May filter requested libraries and must disable producer mode in the exported copy |
| Project bundle install | Validate the source, rewrite only `metadata.name` to the target project slug, then validate again |
| Runtime sync | Copy validated runtime and bootstrap intent into a separate runtime bundle |
| Pipeline hit | Read and validate once at the invocation boundary; do not parse this file for every node |

The serialization lock protects updates made through one Zebflow process.
Operators must not edit the same file concurrently through a separate process.
Git synchronization and project replacement are separate multi-file operations
and must provide their own project-level staging and rollback.

## Version Rules

For `zebflow.com/v1`:

- A patch may fix code or strengthen rejection of content that was already
  invalid. It must not rename fields, change defaults, or change valid meaning.
- A minor Zebflow release may add runtime behavior that does not change this
  document.
- A new field, removed field, renamed field, changed type, changed default, or
  changed meaning requires a new contract API version and an explicit converter.
- The v1 reader never guesses or silently converts another shape.

### Before first release

Zebflow has not published a v1 release. No `zebflow.yaml` exists outside this
repository and its test instances, so there is no document in the world for a
schema change to break. Until first release, `zebflow.com/v1` may be amended in
place, and every amendment is recorded below with its date. A converter is not
written for a document that never existed.

After first release this section closes and the rules above apply literally: the
schema stops being a draft and starts being a promise.

### Amendments

| Date | Change | Why it was safe |
| --- | --- | --- |
| 2026-08-20 | Added `spec.layout` | Optional at every level; omission reproduces the previous hardcoded directories exactly; no existing field changed meaning; absent stays absent on rewrite, so an existing file is not modified on its next save; the golden fixture still round-trips byte for byte. |
| 2026-08-20 | Added `spec.layout.sqlite_schema` | The SQLite export directory was the one repository directory with no entry, so `schemas/sqlite/` stayed a literal in two places that could drift from each other. Optional, defaults to the literal it replaces, and absent stays absent on rewrite. |
| 2026-08-20 | Added `spec.layout.allowed_extensions` | Optional; an absent entry resolves to the platform set, which is derived from what a repository in this codebase actually holds plus the media types the asset route serves, so no existing content becomes uninstallable and absent stays absent on rewrite. It can only narrow, so no document can weaken the gate built on it. No existing field changed meaning and the golden fixture still round-trips byte for byte. |
| 2026-09-01 | Added `spec.files.backend` | Optional; an absent entry resolves to `zebfs`, the store every project already used, and absent stays absent on rewrite, so no existing file is modified on its next save. No existing field changed meaning and both golden fixtures still round-trip byte for byte. The only accepted value is the one a stored FileRef already carries, so nothing written before the entry existed becomes unreadable. |
| 2026-08-20 | `spec.layout.assets` now defaults inside the resolved `source` | It only differs for a project that declares a different `source` -- a case that could not arise before the declaration was read, and where the previous literal would have scaffolded an asset directory in the tree the project moved out of. |
| 2026-09-03 | `spec.layout.assets` renamed to `spec.layout.static`, default `static` | `assets` was the wrong word: in Vite, Astro, Rails and Hugo it names files that are processed and content-hashed, while this directory is served byte-for-byte -- which those same projects call `static`. `public` was not taken because it is the *state* the exposure inventory reports, and a folder claiming that word would collide with it. |
| 2026-09-03 | `spec.layout.source` defaults to the repository itself | It was `pipelines`, and every layout entry was also a directory the platform created on every request -- so a folder an author deleted returned on the next page load. A project now starts as an empty repository and a layout entry only says where a kind is looked for. An empty declaration is legal and names the repository root; `static` is the one entry that may not be empty, because its route serves everything beneath it unauthenticated. |

## Freeze Evidence

- The complete v1 fixture decodes and re-encodes byte for byte, and still does
  so with no `layout` key present.
- A declared layout fixture decodes and re-encodes byte for byte, including a
  narrowed `allowed_extensions` list.
- A declared extension list may narrow the platform set; a wider one, a
  repeated one, an empty one, and one spelled with a dot or in uppercase are
  each refused rather than clamped.
- A package carrying a `.sh` or a `.dylib` is refused by both install gates
  with the offending path and extension named, and a package of ordinary
  project content -- pages, styles, pipelines, images, `zebflow.yaml`,
  `zeb.lock`, `.gitkeep` -- installs unchanged.
- An undeclared layout resolves to the directories the platform hardcodes,
  including the installer's own initial-data prefix table.
- An undeclared `files.backend` resolves to the local store, is not written on
  rewrite, and stores and serves a file exactly as before; a project declaring
  `zebfs` explicitly round-trips byte for byte and resolves to the same store;
  an unknown backend is refused by name with the accepted list rather than
  falling back; and the declaration survives an unrelated settings write.
- A declared layout survives the runtime model conversion and an unrelated
  configuration update, so no settings write can erase it.
- A project that declares no layout is not rewritten to carry an empty section.
- A declared `source` moves registration, discovery, the template root, and the
  asset route together, and a project that declares nothing keeps the same
  directories and the same behavior it had.
- Pipeline identity is source-relative, ids written before that are read through
  the same rule without migrating, and the migration that rewrites them is
  refused rather than guessed when two ids would collapse into one.
- Wrong kind, future version, unknown fields, malformed YAML, and unsafe YAML
  features have negative tests.
- Complete legacy data maps through a dedicated legacy type.
- Failed migration leaves the legacy source untouched.
- Successful migration keeps the original bytes as a recovery copy and reopens
  the canonical file before removing the legacy source.
- Concurrent updates to independent sections are preserved.
- Library, CLI, Hub, transfer, settings, and runtime consumers compile against
  the same service and filename constant.

See [migration.md](./migration.md) for the only supported legacy conversion.
