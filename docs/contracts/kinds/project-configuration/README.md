# ProjectConfiguration

Status: **Frozen** on 2026-08-15. Amended 2026-08-20 to add optional
`spec.layout`; see [Version Rules](#version-rules).

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
| `spec.layout` | Repository-relative directories the project's own files live in | Absolute paths, platform data directories, or file contents |
| `spec.rwe` | RWE policy and requested libraries | Resolved library files or integrity hashes |
| `spec.pipelines` | Pipeline logging retention and node timeout | Pipeline definitions or invocation records |
| `spec.runtime` | Portable execution and resource intent | Worker IDs, pod names, or live allocation state |
| `spec.bootstrap` | Project-relative activation paths | Runtime snapshots |
| `spec.git` | Remote URL, branch, and credential reference | Git passwords, tokens, or private keys |
| `spec.assistant` | Credential references and bounded behavior | API keys or chat history |
| `spec.locks` | Project-relative protected template paths | Access-control policy or file contents |
| `spec.data` | Reserved future data policy | Records, schema, or connection secrets |
| `spec.files` | Upload size policy | Uploaded bytes or object metadata |
| `spec.distribution` | Hub publishing intent | Hub access tokens or published package content |

Credential and connection IDs are references. Their secret values remain in the
project credential store.

`spec.rwe.libraries` is the requested project state. `zeb.lock` owns resolved
entry paths and integrity hashes. They may repeat a requested version and source
for verification, but they do not own the same decision.

### spec.layout

Every entry is one directory, relative to `repo/`, and every entry is optional.
An absent entry means the default in [Frozen Defaults](#frozen-defaults), so a
project that declares no layout describes exactly the directories the platform
has always used.

`source` is one directory, not two: it is the root that holds pipelines, pages,
stylesheets, and shared components, and it is the same directory the RWE
compiler treats as its template root and `@/` import root. `assets` defaults
inside `source` because that is where the asset routes look today.

`initial_data` is an ordered list of prefixes an install would replay, each
bound to the database engine that would replay it. Declaring the list replaces
the default list rather than extending it, so a project that names one prefix
has exactly one.

The declaration is readable today and is not yet consulted. Discovery,
placement, and pipeline identity still use the hardcoded directories, so
declaring a layout that differs from the defaults changes nothing at runtime.
See [layout.md](./layout.md) for the survey of those hardcoded rules.

## Integration Map

Every v1 section has one writer path and one runtime owner. Settings controls
write through `ProjectConfigurationService`; they never edit YAML with direct
file operations.

| Section | User-facing writer | Runtime reader or effect |
| --- | --- | --- |
| `spec.profile` | Project creation and Settings > General | Project lists, headers, and app metadata |
| `spec.layout` | No writer; hand-authored in `repo/zebflow.yaml` | None yet. The declaration is readable through `ZebflowJson::layout()`; no consumer reads it, and the directories remain hardcoded |
| `spec.rwe` | Settings > Policy and Settings > Libraries | RWE compilation, rendering, assets, and editor libraries |
| `spec.pipelines` | Settings > General and Settings > Logs | Node timeout and bounded invocation retention |
| `spec.runtime` | Project creation and explicit project configuration edits | Runtime synchronization and placement planning |
| `spec.bootstrap` | Project package installation and the `zebflow run` flow | Pipeline activation during materialization and repo refresh |
| `spec.git` | Settings > General Git Remote | Git sync and push target selection |
| `spec.assistant` | Settings > Automatons | Project assistant credentials and execution bounds |
| `spec.locks` | Template editor lock control | REST, MCP, and assistant template access checks |
| `spec.data` | Reserved; no writer in v1 | No runtime effect in v1 |
| `spec.files` | Settings > General Runtime Defaults | Asset, file, and webhook upload limits |
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
| `spec.layout.source` | `pipelines` |
| `spec.layout.assets` | `pipelines/assets` |
| `spec.layout.docs` | `docs` |
| `spec.layout.schema` | `schemas/sekejap` |
| `spec.layout.node_interfaces` | `nodes` |
| `spec.layout.initial_data` | `initial-data/sekejap`, `initial-data/sqlite`, `init/sekejap`, `init/sqlite`, `seeds/sekejap`, `seeds/sqlite`, each bound to the engine named in its own path |
| `spec.rwe.minify_html` | `false` |
| `spec.rwe.strict_mode` | `true` |
| `spec.pipelines.logging.max_invocations` | Runtime default of 20 |
| `spec.pipelines.node_timeout_secs` | Runtime default of 30 seconds |
| `spec.runtime.mode` | `shared` |
| `spec.runtime.execution` | `resident` |
| `spec.runtime.resource_profile` | `small` |
| `spec.runtime.min_replicas` | `1` |
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
- repeated initial-data prefixes, and initial-data engines other than `sekejap`
  and `sqlite`
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

## Freeze Evidence

- The complete v1 fixture decodes and re-encodes byte for byte, and still does
  so with no `layout` key present.
- A declared layout fixture decodes and re-encodes byte for byte.
- An undeclared layout resolves to the directories the platform hardcodes,
  including the installer's own initial-data prefix table.
- A declared layout survives the runtime model conversion and an unrelated
  configuration update, so no settings write can erase it.
- A project that declares no layout is not rewritten to carry an empty section.
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
