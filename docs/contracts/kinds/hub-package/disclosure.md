# What a user is told before they consent

Status: **survey**. This describes what the code does now. Every claim below was
checked against the code as written, not inferred from a previous document.
Section 1 was rewritten when the project-bundle review landed; sections 2 and 3
still hold.

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

## 2. The review knows more than it shows

`PackageSafetyReview` carries thirteen findings. Counting how many Zeb templates
render each:

| Finding | Templates rendering it |
| --- | --- |
| `warnings` | 8 |
| `schedules` | 5 |
| `risk_level` | 5 |
| `external_urls` | 4 |
| `database_effects` | 4 |
| `filesystem_effects` | 4 |
| `nodes_used` | 3 |
| `credentials_required` | 3 |
| `public_endpoints` | 3 |
| `large_files` | 2 |
| `seed_data` | 2 |
| `violations` | 2 |
| **`database_initialization`** | **0** |

Only two templates show a safety review at all:
`pages/project-studio/hub/page.tsx` and
`pages/project-studio/pipelines/registry/components/registry-install-catalog.tsx`.

`database_initialization` is the per-file record of what install-time SQL will
do — statements by verb, tables touched, destructive work quoted back, and
whether an existing database is reachable. It is the single most informative
thing the review produces and it is rendered nowhere.

`violations` appearing in only two templates matters more than the number
suggests: a violation is not overridable, so a user who cannot see one cannot
understand why an install refused.

## 3. Consent exists in the API and not in the UI

The install request carries three flags, each defaulting to true:

| Flag | Effect | Templates referencing it |
| --- | --- | --- |
| `include_code` | install source, or only the data model | 0 |
| `include_schema` | write the schema and seed `.sql` files | 0 |
| `execute_schema` | run them, or leave them for the user to run | 0 |

`target_folder` appears in 5 templates, so the install form is not the obstacle;
these three were simply never surfaced.

`execute_schema: false` is the case worth having — the schema lands in the
repository and nothing runs — and no user can currently reach it.

## 4. What follows

Disclosure is a chain. The first link now holds: every install surface, the
project bundle included, can be asked what it would do before it does it.

The remaining two links are unchanged. `database_initialization` is still
rendered by no template, so the richest finding the review produces reaches no
user through the UI; and the three consent flags are still reachable only by
calling the API directly. The project-bundle review endpoints are listed in the
`hub_api` map the home and hub pages already receive, so the data is in reach of
the templates that would show it, but no template calls them yet.
