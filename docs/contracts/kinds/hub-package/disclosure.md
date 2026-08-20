# What a user is told before they consent

Status: **survey**. This describes what the code does now. Every claim below was
checked against the code as written, not inferred from a previous document.
Section 1 was rewritten when the project-bundle review landed; sections 2 and 3
were rewritten when the review and the consent choice reached the UI, and
section 2 again when the findings stopped being guessed from node kind strings.

Zebflow's stated goal is to be the most informative dependency scanner there is:
a user should finish reading an install review knowing exactly what will happen
to their data, and should be able to consent to parts of it.

This measures the distance to that.

## 1. Every install surface can be previewed

| Surface | Pre-install review |
| --- | --- |
| Hub asset / pack | `POST .../hub/assets/{package_id}/{version}/review` |
| Remote repository pack | `POST .../hub/repositories/{id}/packs/{pkg}/{ver}/review` |
| Node bundle | `POST .../nodes/install/review` |
| UI component | `POST .../install/ui/review` |
| Project bundle | `POST /api/platform/hub/install/review` |
| Project bundle, per account | `POST /api/users/{owner}/hub/install/review` |

The project-bundle row was empty when this survey was written, and that path is
the most consequential in the system: it creates a project, writes the bundle's
files, applies both schema engines, executes the bundle's seed SQL, registers
every pipeline it wrote, and activates the ones the bundle names.

Its review takes the same body as the install, consent flags included, and
answers with the destinations that would be written, the pipelines that would be
registered and which of them activated, the pipelines the bundle names active
that this install would *not* activate, `database_initialization`, the safety
review, `installable`, and what the current flags would skip. Review and install
build one `ProjectBundleInstallPlan` from the same inputs, so a package the
review calls installable is one the install installs, entry for entry.

## 2. What the review knows, the UI now shows

`PackageSafetyReview` carries fifteen findings. Three templates render a
package safety review, and all three now render `database_initialization`:

| Template | Review it renders |
| --- | --- |
| `pages/hub/page.tsx` | project bundle, `ProjectBundleInstallReview` |
| `pages/project-studio/hub/page.tsx` | node bundle from a file, `HubInstallReview` |
| `pages/project-studio/pipelines/registry/components/registry-install-catalog.tsx` | hub pack add, `HubInstallReview` |

`database_initialization` is the per-file record of what install-time SQL will
do — statements by verb, tables touched, destructive work quoted back, and
whether an existing database is reachable. It is rendered by
`components/package-review/sql-report.tsx`, which renders every field the report
carries: `engine`, `source`, `store`, `existing_data_at_risk`, the full
statement breakdown including `OTHER`, `tables`, every quoted destructive
statement, and `unreadable` when the bytes could not be read.

`existing_data_at_risk: false` with `store: "created by this install"` is stated
rather than implied, because it is the most reassuring thing the report can say:
sekejap and sqlite stores are created *by* the install, so bundle SQL cannot
reach a database the user already had. An engine this build does not describe
reports the opposite, in red.

Every finding that names a node kind is now derived from the node catalog
rather than matched against the kind string. `database_effects` and
`filesystem_effects` kept their names and changed their evidence; two more
joined them, `network_effects` and `code_execution`, and both report something
no name could have revealed. A node whose URL is assembled from a credential or
a placeholder contributes nothing to `external_urls` and everything to
`network_effects`, so a package can no longer make outbound calls while
reporting no destination at all.

Nothing in the package declares any of this. A native node's capabilities are a
compile-time enum; a composite node's are the union of the nodes its function
pipelines compose, and those pipelines are now reviewed like any other; a WASM
node's are empty because the host grants its modules no imports. A kind the
review cannot account for is named in the warnings, because a node quietly
skipped would understate the package.

One violation reads the package rather than judging it: a bundle declaring a
node kind this build already provides. The kind is in the bundle's own manifest,
the native catalog is fixed at compile time, and such a bundle makes the whole
project's node registry fail to load. Nothing else about a capability is a
violation — using the network is not a crime, and a refusal nobody can override
has to be provable from the package rather than inferred from its power.

`violations` and `warnings` are rendered by
`components/package-review/findings.tsx` and are deliberately different in kind
and in shape. A warning is dashed amber, is introduced as something installing
accepts, and blocks nothing. A violation is a solid two-pixel red panel with
`role="alert"`, a filled BLOCKED chip, and a sentence saying it cannot be
accepted or overridden by anyone; the confirm button beside it is disabled and
reads "Blocked". A user facing a refusal can read exactly why.

## 3. Consent is reachable from the UI

The install request carries three flags, each defaulting to true:

| Flag | Effect |
| --- | --- |
| `include_code` | install source, or only the data model |
| `include_schema` | write the schema and seed `.sql` files |
| `execute_schema` | run them, or leave them for the user to run |

`components/package-review/install-consent.tsx` renders all three as checkboxes
in the Hub install flow (`pages/hub/page.tsx`), each with the consequence of the
state it is currently in, not a description of the flag. Turning
`include_schema` off turns `execute_schema` off with it and locks the control,
because `execute_schema: true` with `include_schema: false` is a request error
rather than a smaller install; a scope with neither code nor schema is refused
in the form for the same reason. Changing any flag marks the report stale and
says so, because the review answers for the flags it was handed.

`execute_schema: false` is the case worth having — the schema lands in the
repository and nothing runs — and it is now reachable: the review names the
files under "SQL written, left for you to run", and the install reports how many
are waiting.

## 4. What remains

The chain holds end to end for the project bundle: it can be asked what it would
do, the answer is shown in full, and the consent flags that shape the answer are
the ones the install is given.

Two gaps are recorded and not closed. `UiInstallReview` has no `violations`
field at all, so a UI component install has no non-overridable refusal to show.
And the project-studio Hub page's "Add" button for a hub pack installs without a
review; the reviewed path for the same package is Add+ in the pipeline registry.
