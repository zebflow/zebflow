# Formal Contract of Project Distribution

Keep this file super lean with strong formal logic

## Project Storage Model

A Zebflow project is an isolated workspace rooted at:

- `users/{owner}/{project}/`

Within that root, Zebflow uses three primary storage areas:

- `repo/`
- `data/`
- `files/`

These three roots are the formal project storage model. All project distribution
mechanisms should be understood in terms of which of these roots they include,
exclude, lock, copy, or rebuild.

### `repo/`

`repo/` is the source workspace of the project.

It is the editable, versionable definition of the app and is the primary target
for Git-based operations.

Key contents:

- `.git/`
- `pipelines/`
- `docs/`
- `zebflow.json`

Important notes:

- `repo/pipelines/` is the unified source root for pipeline definitions,
  templates, scripts, and related project code.
- default subdirectories under `repo/pipelines/` include:
  - `assets/`
  - `styles/`
- `repo/` is the main source-of-truth area for project logic and structure.

### `zebflow.json`

`zebflow.json` is the project manifest stored at:

- `repo/zebflow.json`

It belongs to the `repo/` storage area because it is part of the project
definition, not runtime state.

Formal role:

- stores project-level configuration
- stores project metadata used by the platform and studio
- acts as the canonical manifest for project defaults and locked project rules

Actual top-level structure:

it should be
{
  metadata: {
    title: {}
    description: {}
  }
  configs: {
    rwe: {}
    pipelines: {
      nodes: {}
    }
    runtime: {}
    bootstrap: {}
    git: {
      remote: {} 
    }
    assistant: {}
    locks: {
      templates: []
    }
    data: {}
    files: {
      uploads: {
        max_asset_size_mb: {}
        max_file_size_mb: {}
      }
    }
  }
  distribution: {
    marketplace: {}
  }
}

Current settings coverage under the proposed shape:

| Path | Meaning |
| --- | --- |
| `metadata.title` | Human-facing project title. Currently stored today as `project.title`. |
| `metadata.description` | Project description. Currently stored today as `project.description`. |
| `configs.rwe.allow_list` | Allowed external script/style URL patterns for RWE. |
| `configs.rwe.minify_html` | Whether rendered HTML is minified. |
| `configs.rwe.strict_mode` | Whether strict compile/runtime checks are enabled. |
| `configs.rwe.libraries` | Enabled `zeb/*` library declarations for the project. |
| `configs.rwe.deployment_asset_base` | Base asset prefix override for compiled RWE assets. |
| `configs.pipelines.logging.max_invocations` | Pipeline invocation retention limit. Currently stored today as `logging.max_invocations`. |
| `configs.pipelines.nodes` | Reserved place for project-level node settings. No durable per-project node config is stored yet. Current Nodes settings page is built from built-in registry data. |
| `configs.runtime` | Portable runtime profile for runtime mode / placement. Currently stored today as top-level `runtime`. |
| `configs.bootstrap` | Project bootstrap or activation plan. Currently stored today as top-level `bootstrap`. |
| `configs.git.author_name` | Git author display name for project commits. |
| `configs.git.author_email` | Git author email for project commits. |
| `configs.git.remote.credential_id` | Credential id used for authenticated Git remote access. |
| `configs.git.remote.repo_url` | Git remote repository URL. |
| `configs.git.remote.branch` | Default Git remote branch. |
| `configs.assistant.high_model_credential` | Credential id for higher-tier assistant model use. |
| `configs.assistant.general_model_credential` | Credential id for general assistant model use. |
| `configs.assistant.max_steps` | Assistant execution step cap. |
| `configs.assistant.max_replans` | Assistant replan cap. |
| `configs.assistant.chat_history_pairs` | Assistant chat history retention for project chat. |
| `configs.assistant.enabled` | Whether project assistant usage is enabled. |
| `configs.locks.templates` | Locked template paths or folder prefixes. |
| `configs.data` | Reserved place for durable project-level data configuration. No dedicated `zebflow.json` data section is stored yet. |
| `configs.files.uploads.max_asset_size_mb` | Max upload size for one asset file. Currently stored today as `assets.max_asset_size_mb`. |
| `configs.files.uploads.max_file_size_mb` | Reserved place for broader file upload policy. Not stored yet in current `zebflow.json`. |
| `distribution.marketplace` | Reserved place for project marketplace/distribution contract. Not stored yet in current `zebflow.json`. |

Important note:

- current persisted settings already cover:
  - metadata
  - RWE
  - assistant
  - Git identity
  - Git remote
  - pipeline log retention
  - runtime profile
  - bootstrap plan
  - asset upload policy
  - template locks
- current Settings UI also has surfaces that are not yet durable `zebflow.json`
  domains, for example:
  - node registry view
  - Git repository health / repair
  - re-index / recovery actions
  - project transfer operations
- `zebflow.json` is Layer 2 project config:
  - git-synced
  - non-sensitive
  - part of the portable project definition

Important note:

- if a project is distributed through Git sync, marketplace, or project export,
  `repo/zebflow.json` is part of the project definition and should be treated as
  first-class source content.

### `data/`

`data/` is the project-local runtime state area.

It stores mutable operational state that belongs to the project runtime, not the
versioned source workspace.

Key contents:

- `data/runtime/`
- `data/runtime/pipelines/`

Important notes:

- `data/` is not the same as `repo/`.
- `data/` is for local runtime state, generated state, and adapter-managed
  project data.
- project-scoped database/runtime adapter state may also live under `data/`.

### `files/`

`files/` is the project file-storage area.

It is used for stored file payloads that are not part of the source workspace.

Key contents:

- `files/public/`
- `files/private/`

Important notes:

- `files/public/` contains files that may be exposed through public interfaces.
- `files/private/` contains files intended for internal/project-only use.
- `files/` is distinct from `repo/pipelines/assets/`.
  - `repo/pipelines/assets/` belongs to the source workspace.
  - `files/` belongs to project file storage.

### Formal Role Split

The three roots have different responsibilities:

- `repo/` = source definition
- `data/` = runtime state
- `files/` = stored file payloads

This split is the first formal basis for Git sync, backup, transfer, and
marketplace behavior.

## Distribution Capabilities

This section compares the current distribution mechanisms by what they are
meant to move.

Legend:

- `✓` supported directly
- `~` partial / conditional / evolving
- `-` not the intended mechanism

| Capability | Git | Marketplace | Import / Export | Transfer |
| --- | --- | --- | --- | --- |
| Versioned source code in `repo/` | ✓ | ~ | ✓ | ✓ |
| Pipelines only | ~ | ✓ | ~ | ~ |
| Templates / UI pieces only | ~ | ✓ | ~ | ~ |
| Reusable packs / partial project material | - | ✓ | ~ | ~ |
| Full project-wide source workspace | ✓ | ~ | ✓ | ✓ |
| Runtime `data/` | - | - | ✓ | ✓ |
| Stored `files/` payloads | - | ~ | ✓ | ✓ |
| DB structure shipping | ~ | ~ | ✓ | ✓ |
| DB init / seed shipping | - | ~ | ✓ | ✓ |
| Forkable public distribution | ~ | ✓ | - | - |
| Remote sync / push workflow | ✓ | ~ | - | ~ |
| Backup / restore workflow | ~ | - | ✓ | ✓ |
| Instance-to-instance movement | ~ | ~ | ✓ | ✓ |

### Meaning

#### Git

Git is the source sync and versioning mechanism for the project workspace.
It is authoritative for `repo/`, but it is not the mechanism for moving
runtime `data/` or stored `files/`.

#### Marketplace

Marketplace is the share, clone, and reuse mechanism. It is strongest for
packs, reusable project material, and forkable project distribution. It is not
the primary mechanism for moving full runtime state.

##### Authority Model

Marketplace authority remains project-based.

Formal marketplace authority URL:

- `/api/projects/{owner}/{project}/marketplace`

This means a marketplace authority is hosted by one project, not by the
platform Home surface.

However, marketplace producer behavior is not open by default.

Formal producer rule:

- project marketplace producer mode is disabled by default
- ordinary projects may consume marketplace packages without becoming producers
- only explicitly enabled projects may expose producer APIs

Current target policy:

- only curated `superadmin`-owned projects may enable marketplace producer mode

This keeps current Zebflow marketplace URLs and storage intact while avoiding a
world where every project becomes a public marketplace authority.

##### Consumer vs Producer

Marketplace use splits into two modes:

- consumer
- producer

Consumer meaning:

- browse configured marketplace sources
- install project/app packages
- install project-scoped reusable packs

Producer meaning:

- host a marketplace authority URL
- issue marketplace tokens
- publish packages
- manage publisher identities

Formal rule:

- consumer access may be broad
- producer capability must be explicitly enabled and curated

##### Identity Model

Marketplace should not treat platform users as public publisher identity.

Formal identity split:

- `platform user`
  - authenticated internal actor
  - used for login, access control, and audit trail
- `producer project`
  - project that hosts one marketplace authority
- `publisher_id`
  - stable public publishing identity inside that marketplace authority
- `token`
  - revocable credential that authorizes publish/read/manage operations

Formal rule:

- a platform user does not automatically become a marketplace publisher
- publishing should happen as one explicit `publisher_id`
- tokens should be bound to publisher identity, not used as the identity itself

This allows token rotation or revocation without changing the public publisher
identity.

##### Publisher Contract

Inside one producer marketplace authority, publisher identity should be formal
and stable.

Minimum publisher fields:

- `publisher_id`
- `display_name`
- `publisher_url`
- `email`
- `description`
- `icon_url`
- `website_url`
- `created_at`
- `updated_at`
- `enabled`

Formal meaning:

- `publisher_id`
  - stable immutable public identifier
- `display_name`
  - human-facing alias, for example `Zebflow Official`
- `publisher_url`
  - stable public publisher route inside that marketplace authority
- `email`
  - stable publisher contact channel for support, revocation requests, and trust
- token
  - one revocable credential that can publish as that publisher

This means a leaked or expired token can be replaced while the same publisher
alias and publisher URL remain stable.

##### Package Display and Attribution

Marketplace packages should support display-oriented metadata without changing
the authority or publish-token mechanism.

Formal rule:

- package display metadata is optional
- package display metadata is not the same as package authority
- documentation and attribution content may be shown in marketplace list/detail
  views

Recommended package display fields:

- `readme_ref`
- `license_ref`
- `attribution_text`
- `authors`
- `homepage_url`

Formal meaning:

- `readme_ref`
  - a repo-relative documentation reference, for example `README.md`
- `license_ref`
  - a repo-relative license or attribution document reference
- `attribution_text`
  - short human-readable attribution shown directly in marketplace surfaces
- `authors`
  - author list for package/project provenance
- `homepage_url`
  - external canonical project or publisher landing page

Recommended convention for project bundles:

- if a root `README.md` exists, marketplace may use it as the default
  `readme_ref`

Formal purpose:

- preserve attribution when projects are installed, forked, or republished
- show usage notes and context in marketplace search/detail surfaces
- keep provenance visible without changing package installation behavior

##### Producer Activation

Enabling producer mode is a sensitive action and should be treated similarly to
other destructive or exposure-enabling project operations.

Formal rule:

- producer mode activation should require privileged confirmation
- producer mode should not be enabled accidentally during normal project setup

##### Visibility Model

Project visibility and marketplace visibility are different concerns.

Formal rule:

- platform Home shows projects visible to the current user
- marketplace browsing shows packages visible from configured marketplace
  sources

This means a user may browse marketplace packages without automatically seeing
all projects on the platform, and may see local projects without owning any
publisher identity.

##### Share Types

Marketplace should distribute typed packages, not arbitrary file dumps.

Formal share types:

- `project`
- `folder`
- `pipeline`
- `template`
- `script`

Formal meaning:

- `project`
  - a full project-level source package
  - may be installable as an app or as a general project
- `folder`
  - a typed reusable subtree of `repo/`
  - should not mean an arbitrary folder dump
- `pipeline`
  - one pipeline plus its required internal repo dependencies
- `template`
  - one template plus its required internal repo dependencies
- `script`
  - one script/module plus its required internal repo dependencies

##### Packaging Rules

Every marketplace package should have:

- one formal `kind`
- one formal `root_ref`
- one dependency-aware package closure

Dependency rule:

- if a package kind is smaller than a full project, it should still include the
  required internal `repo/` dependency closure needed for installation and use

Default exclusions:

- `data/`
- credentials and secrets
- `.git/`
- caches
- logs
- user-local state

Optional attached payloads:

- `files/`
- DB schema
- DB init / seed
- docs
- package metadata and media

Formal principle:

- Marketplace distributes portable source packages.
- Runtime state is excluded by default unless a payload type is explicitly
  defined for it.

#### Import / Export

Import / export is the archive-based project movement path. It is the current
explicit mechanism for packaging and restoring broader project state, including
project-wide source and runtime data.

#### Transfer

Transfer is the broader project movement concept across offices, environments,
or instances. Import / export is one concrete implementation path of transfer.

#### Compile
