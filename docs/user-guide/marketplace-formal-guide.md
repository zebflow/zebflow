# Zebflow Marketplace Formal Guide

Draft status: review draft.

This guide defines the intended stable model for the Zebflow marketplace.

The marketplace should be:

- closed by default
- platform-scoped and office-hosted, with superadmin-only enablement for the
  shippable version
- curated
- typed
- dependency-aware
- safe to consume without becoming a producer
- explicit about publisher identity, token scope, and trust boundaries

The marketplace should not become a hidden public project browser, an arbitrary
file dump, or a way to bypass project permissions.

## 1. First Principles

Marketplace exists to move reusable Zebflow work between projects and offices.

It is not the same thing as:

- project visibility
- Git sync
- backup and restore
- runtime state migration
- public anonymous file hosting

Formal principle:

> Marketplace distributes explicit packages through an explicit marketplace
> authority. It does not expose projects automatically.

## 2. Marketplace Authority

A marketplace authority is a platform service instance hosted by one Zebflow
office.

Formal authority URL:

```text
/api/projects/{owner}/{project}/marketplace
```

This legacy/internal route shape exists in the current implementation, but it is
not the long-term authority model.

Formal authority model:

```text
service_instance_id: marketplace-default
service_kind: marketplace
display_label: Marketplace
host_office_id: {office_id}
state_office_id: {office_id}
public_base_url: https://market.zebflow.com/api
enabled: true
status: online
placement_generation: 1
```

This means the marketplace service is platform-scoped and office-hosted. It is
not a normal project.

Formal functions:

```text
service_host(marketplace-default) = {office_id}
state_host(marketplace-default) = {office_id}
service_manager(marketplace-default) = root({office_id})
```

Base rule:

> In the base model, marketplace runtime and marketplace state live in the same
> host office. Controller governance must not move the marketplace database into
> the controller.

Public marketplace URL:

```text
https://market.zebflow.com/api
```

The public URL is an opaque authority alias. It must not require the consumer to
know or enter the internal owner/project slugs.

The platform Home surface can browse marketplace sources, but Home is not itself
the authority. Home is a consumer surface.

Formal rule:

- one marketplace service instance belongs to one host office
- marketplace service is not a project
- marketplace service enablement is platform-level
- marketplace service management is disabled by default and superadmin-only
- project-side publishing is token-scoped; a project becomes a producer only
  when it has a valid publisher token/profile for that marketplace
- package identity is scoped to one marketplace service instance
- public consumers use marketplace aliases, not internal owner/project
  coordinates

Formal package identity:

```text
{service_instance_id}/{package_id}
```

Formal version identity:

```text
{service_instance_id}/{package_id}/{version}
```

Formal rule:

> `package_id` is only unique inside one marketplace authority. It must never be
> used as a global lookup key by itself.

Implementation rule:

- package reads must include service instance id
- package writes must include service instance id
- version reads must include service instance id and package id
- install flows must preserve source marketplace identity
- imported remote packages must not overwrite unrelated marketplace packages

Office placement rule:

> Marketplace is embodied in its host office. The marketplace operational
> database, package artifacts, media assets, token hashes, quotas, and local
> service audit records live under the host office while that office hosts the
> marketplace service.

State ownership rule:

> `state_office_id` must equal `host_office_id` for marketplace v1. A controller
> may keep inventory and replicated summaries, but those summaries are not the
> source of truth.

Controller rule:

> A controller may govern marketplace placement and management, but it does not
> become the owner of the marketplace runtime state unless it is also the
> marketplace host office.

Privacy rule:

> `{owner}/{project}` is an internal authority coordinate. Public marketplace
> consumers should only see an opaque marketplace base URL, repository title,
> package identity, and curated publisher identity.

Alias rule:

> A public marketplace proxy such as `https://market.zebflow.com/api` maps to one
> internal authority without exposing its owner or project slug in the consumer
> setup flow.

Public consumer metadata must not expose:

- authority owner slug
- authority project slug
- source owner slug
- source project slug
- platform username behind a publisher

Public consumer metadata may expose:

- marketplace alias
- repository title
- package id
- package version
- package title and description
- curated `publisher_id`
- curated publisher display name
- curated publisher URL

Public API projection rule:

> Remote/public marketplace list, detail, search, and install-preview APIs must
> return a public projection of package metadata, not the raw internal package
> row or internal artifact manifest.

Public API responses must not include fields such as:

- `authority_owner`
- `authority_project`
- `publisher_owner`
- `source_owner`
- `source_project`
- internal project id
- internal user id
- filesystem path
- platform username behind a curated publisher

## 3. Who Can Become A Marketplace

Current policy:

- only superadmin can enable and manage the platform marketplace service
- normal user projects can consume marketplace packages
- normal user projects can publish only through publisher tokens/profiles
- normal user projects do not become marketplace service hosts by accident

Current enablement mechanism:

- the current platform session must be superadmin
- the user must confirm their password
- the user must select a host office
- the user must set or confirm the public marketplace base URL
- the marketplace service placement row is enabled only after those checks pass

Ownership decision:

The marketplace service belongs to the selected host office for runtime and
state. It belongs to the platform/controller for governance while the office is
federated.

The product policy for the shippable version should be:

> Only superadmin can enable the marketplace service and assign it to an office.

This means `service_instance_id` is the authority identity. Project
owner/project slugs are not part of public marketplace identity.

Formal rule:

> Marketplace service activation is a privileged exposure event. It must require
> explicit confirmation and must not happen as a side effect of project creation,
> package installation, or repository browsing.

## 4. Closed-By-Default Route Model

Marketplace has two broad route classes:

- consumer routes
- producer routes

Consumer routes allow browsing and installing from configured sources.

Producer routes allow the authority to:

- list publish sources
- preview packages
- publish packages
- manage publishers
- create and revoke tokens
- serve remote marketplace APIs

Formal route rule:

- management routes must check superadmin authority and marketplace service
  state where applicable
- project publish-source, preview, and install routes must check project
  capability
- publish routes must require `marketplace:publish` tokens and derive publisher
  identity from the token
- remote producer APIs must require marketplace bearer tokens where appropriate
- a disabled authority must not leak publish sources, token lists, publisher lists,
  or private packages

Current project capability used by manager routes:

- marketplace manager operations currently require project write-level authority
  through `PipelinesWrite`

Required design:

- introduce a dedicated `MarketplaceManage` project capability instead of
  reusing `PipelinesWrite`

Formal route capability target:

- consumer browse: `marketplace.read`
- consumer install: `marketplace.install`
- publisher profile use: `marketplace.publish`
- publisher and token management: `marketplace.manage`
- producer enablement: platform superadmin plus password confirmation

## 5. Consumer And Producer Roles

Marketplace usage splits into two roles.

Consumer means:

- configure marketplace repository sources
- browse visible packages
- install a package into a project
- use a read token when a remote source requires one

Producer means:

- operate one enabled marketplace authority
- create publisher identities
- issue publisher tokens
- publish packages
- revoke tokens
- disable publishers

Formal rule:

> A project can be a consumer without being a producer.

Formal rule:

> A producer is curated through marketplace management. A random project owner or
> package author does not automatically receive producer power.

## 6. Publisher Identity

Marketplace identity has three layers:

- platform user
- marketplace authority
- publisher identity

Platform user:

- logs into Zebflow
- has project capabilities
- performs marketplace manager actions
- appears in audit trails

Marketplace authority:

- is the office-hosted marketplace service
- owns publisher records, package records, package versions, and tokens

Publisher identity:

- is the public identity shown on packages
- is stable across token rotation
- is curated by the marketplace manager
- is not the platform user slug
- is not the source project slug

Formal rule:

> A platform user is not automatically a public publisher.

Formal rule:

> A publisher is created by a marketplace manager inside one enabled authority.

Public attribution rule:

> Public marketplace consumers see the curated publisher identity. They must not
> see the backing platform owner/project slugs used to host the marketplace.

Current publisher fields:

- `publisher_id`
- `display_name`
- `publisher_url`
- `email`
- `description`
- `icon_url`
- `website_url`
- `enabled`
- quota fields
- permission fields
- `created_at`
- `updated_at`

Formal publisher meaning:

- `publisher_id` is the stable public identifier
- `display_name` is the human label
- `publisher_url` is the stable public publisher URL or alias path
- `email` is the contact channel
- `enabled` controls whether the publisher may receive new tokens or publish
- quota fields control package count, version count, storage, and media limits
- permission fields control publish, update, and unpublish behavior

## 7. Marketplace Tokens

Marketplace tokens are credentials issued by the marketplace manager for one
publisher inside one authority.

A token is not the publisher identity. A token acts as that publisher.

Current token scopes:

- `marketplace:read`
- `marketplace:publish`
- `marketplace:manage`

Current token behavior:

- token value is shown once at creation
- token secret is stored hashed
- token has a stable `token_id`
- token can have `expires_at`
- token records `last_used_at`
- token can be revoked
- revoked or expired tokens are rejected
- tokens are checked against required scope
- tokens are checked against authority owner/project before remote publish/read
- tokens are checked against publisher status
- tokens are checked against publisher permissions

Formal rule:

> Tokens are revocable scoped credentials. They must never become durable public
> identity.

Formal token lifecycle:

1. Marketplace manager creates or updates a publisher.
2. Marketplace manager creates a token for that publisher with explicit scopes.
3. Marketplace manager gives the token to the producer.
4. Producer uses the token for remote publish or read flows.
5. Marketplace manager revokes the token when it is leaked, expired, rotated, or
   no longer needed.
6. Marketplace manager creates a replacement token if the publisher should keep
   publishing.

Current refresh mechanism:

- token refresh is modeled as revoke old token plus create a new token
- publisher identity remains unchanged

Open design question:

- expose this as a first-class "Rotate token" UI action that performs revoke and
  create in one workflow

## 8. Publisher Limits And Quotas

Publisher limits belong to the marketplace authority's publisher settings.

Formal rule:

> The marketplace manager decides how much a publisher may publish. The
> producer does not choose its own quota.

Recommended publisher quota fields:

- `max_packages`
- `max_versions_per_package`
- `max_package_bytes`
- `max_version_bytes`
- `max_asset_bytes_per_package`
- `max_asset_count_per_package`
- `max_total_storage_bytes`
- `allowed_asset_types`
- `can_publish`
- `can_update`
- `can_unpublish`

Recommended first default:

```text
max_packages: 20
max_versions_per_package: 10
max_package_bytes: 10 MB
max_version_bytes: 10 MB
max_asset_bytes_per_package: 5 MB
max_asset_count_per_package: 8
max_total_storage_bytes: 200 MB
allowed_asset_types: png, jpg, jpeg, webp, gif
can_publish: true
can_update: true
can_unpublish: false
```

Publish checks:

- active published package count must not exceed `max_packages`
- versions per package must not exceed `max_versions_per_package`
- the current package identity must not exceed `max_package_bytes` across
  retained versions
- the current version artifact must not exceed `max_version_bytes`
- package media assets must not exceed `max_asset_bytes_per_package`
- package media asset count must not exceed `max_asset_count_per_package`
- total retained publisher artifact storage must not exceed
  `max_total_storage_bytes`
- every media file must match `allowed_asset_types`
- quota validation must happen before the package is accepted

Package count rule:

> `max_packages` counts active package identities, not historical versions.

Storage rule:

> Historical versions consume storage and must be counted against publisher
> storage quota.

Version retention rule:

> A marketplace authority may keep old versions, but retention must be bounded
> by `max_versions_per_package` or an explicit authority retention policy.

## 9. Content Types

Marketplace must distribute typed content, not arbitrary folders.

Formal content kinds:

- project
- folder
- pipeline
- template
- script

Current source types:

- `project_files`
- `folder_files`
- `pipeline_with_dependencies`
- `template_with_dependencies`

Mapping:

- `project_files` maps to a project package
- `folder_files` maps to a folder package
- `pipeline_with_dependencies` maps to a pipeline package
- `template_with_dependencies` maps to a template package
- script packages are a formal target, but should be verified against current UI
  and service support before being promised as complete

Formal rule:

> Each package must have one declared kind and one root reference.

Package metadata should include:

- `package_id`
- `version`
- `asset_kind`
- `source_type`
- `source_ref`
- `title`
- `description`
- `visibility`
- `tags`
- publisher attribution
- content manifest
- artifact hash
- authority owner/project
- package media manifest
- install manifest
- schema-effects manifest

## 10. Dependency Closure

Marketplace packages should be useful after installation.

Formal rule:

> A package smaller than a full project must include the internal repo
> dependencies required for it to run or render.

Examples:

- a pipeline package should include dependent pipeline files or local modules it
  needs
- a template package should include imported components, local scripts, and
  required project template files
- a folder package should include the typed folder contents under its root
- a project package should include the project source workspace, not runtime
  secrets or mutable data

Formal exclusion rule:

Marketplace package artifacts must exclude:

- credentials
- secret values
- runtime database contents
- `data/`
- `.git/`
- symlinks that resolve outside the source root
- absolute paths
- parent traversal paths
- caches
- logs
- generated temporary files

Path rule:

> Every artifact entry path must be a normalized relative path. Empty paths,
> absolute paths, `..` traversal, drive-letter paths, and backslash traversal
> must be rejected before publish and before install.

## 11. Package Description, Media, And Assets

Marketplace packages need enough media to let consumers inspect what they are
installing, but marketplace media must not become arbitrary public file hosting.

Formal package media layout:

```text
marketplace/
  README.md
  assets/
    icon.png
    screenshot-1.png
    demo.gif
```

Formal rule:

> A published artifact stores package description and media under the
> marketplace service package version root:
> `services/marketplace-default/packages/{package_id}/versions/{version}/`.
> The authoring location inside the source project may be more flexible.

`readme.md` in the package version root is the package detail document. It
should describe:

- what the package does
- supported install modes
- required project capabilities
- required environment variables without secret values
- database schema effects
- runtime routes exposed by an app package
- screenshots or demo images through relative `media/` links

Allowed v1 media types:

- `png`
- `jpg`
- `jpeg`
- `webp`
- `gif`

Disallowed v1 media types:

- `svg`
- executable formats
- HTML
- arbitrary archives

Media safety rules:

- strip image metadata before serving
- validate image type by content, not only extension
- enforce file count and byte quotas from publisher settings
- serve ingested media from marketplace-controlled immutable URLs
- do not hotlink remote README images by default
- normalize and validate media paths
- reject duplicate media paths after normalization
- reject decompression bombs and oversized dimensions
- serve media with a safe content type and `nosniff`

External images:

- HTTPS image URLs may remain visible as ordinary links
- remote images should not be rendered inline by default
- HTTP, localhost, and private-network image URLs must not be fetched server-side

Formal reason:

> External images can change after review, disappear, track consumers, or be
> replaced with unsafe content. Marketplace detail pages should render reviewed
> marketplace-owned media.

README rendering rule:

> Marketplace README content is sanitized Markdown. Raw HTML is not part of the
> v1 rendering contract.

README sanitizer must:

- reject or strip `<script>`, `<style>`, `<iframe>`, event handlers, and raw HTML
- reject `javascript:`, `data:`, `file:`, and private-network URL schemes
- rewrite relative image references to marketplace-owned media URLs
- render external HTTPS image references as links unless explicitly reviewed
- add safe link attributes for external links
- enforce a Content Security Policy on marketplace detail pages

Authoring examples:

```text
src/
  calculator.ts
assets/
  screenshot1.png
  screenshot2.png
```

The user may publish `src/calculator.ts` and select `assets/screenshot1.png`
and `assets/screenshot2.png` as marketplace screenshots. The resulting artifact
is normalized to:

```text
content/
  src/
    calculator.ts
marketplace/
  README.md
  assets/
    screenshot1.png
    screenshot2.png
```

Optional advanced convention:

```text
tools/calculator/
  calculator.ts
  marketplace/
    README.md
    assets/
      screenshot1.png
      screenshot2.png
```

The publish wizard may auto-detect the folder-level `marketplace/` directory,
but manual selection of README and media assets should always be available.

## 12. Visibility

Package visibility and project visibility are different.

Current package visibility values:

- `public`
- `private`
- `unlisted`

Formal rule:

> Home project visibility does not imply marketplace package visibility.

Formal rule:

> Marketplace package visibility does not imply direct access to the source
> project.

Read behavior:

- public packages can be listed by remote consumers
- private packages require an authenticated marketplace token or local authority
  ownership
- unlisted packages do not appear in ordinary browsing
- unlisted packages are fetchable by explicit package/version reference without
  a token unless the authority marks them as token-required

Implementation note:

- if unlisted should be token-gated, it must become a separate visibility value
  or package access policy; do not rely on the word "unlisted" to imply privacy

## 13. Repository Sources

Consumers browse packages through configured repository sources.

Repository source fields:

- `repository_id`
- `title`
- `base_url`
- `authority_alias`
- `remote_owner`
- `remote_project`
- `read_token`
- `enabled`

Field meaning:

- `authority_alias` is the preferred public reference for marketplace sources
- `remote_owner` and `remote_project` are internal/private deployment fallback
  fields
- public hosted marketplace presets must leave `remote_owner` and
  `remote_project` empty

Formal URL rule:

> Remote marketplace sources must be public HTTP(S) URLs.

Blocked targets include:

- localhost
- loopback IPs
- private RFC1918 networks
- link-local networks
- carrier-grade NAT
- multicast and unspecified addresses

Formal reason:

Marketplace remote fetch is an outbound network action. It must not become SSRF
against local infrastructure, cloud metadata, office-private services, or
controller internals.

Direct authority URL form:

```text
https://host/api/projects/{owner}/{project}/marketplace
```

This form is valid only for internal admin use, private deployments, local
development, or an explicitly trusted internal network where exposing project
coordinates is acceptable.

Proxy/base URL form:

```text
https://market.example.com/api
```

When using a registered marketplace alias, the repository must not require
`remote_owner` or `remote_project` from the user. The proxy resolves the alias
to the internal authority.

When using a generic API base that is not a registered alias, the repository may
require `remote_owner` and `remote_project` so the client can construct the
direct authority route. This is an advanced/private-deployment mode, not the
default public marketplace UX.

Consumer privacy rule:

> Outside a bare internal-network direct URL, there should be no consumer-facing
> flow that reveals the authority owner slug, authority project slug, source
> owner slug, or source project slug.

Remote fetch rule:

> Marketplace remote fetch must validate the final request URL and every
> redirect target against egress policy.

Remote response rule:

> Remote artifact responses must be size-limited before full JSON parsing.

## 14. Producer Flow

Formal producer flow:

1. Superadmin opens platform-level Marketplace management.
2. Superadmin enables the marketplace service by selecting the marketplace host
   office and confirming their password.
3. Controller creates or updates `PlatformServiceInstance`.
4. Selected office provisions marketplace service runtime and state.
5. Marketplace manager creates a publisher identity.
6. Marketplace manager sets publisher permissions and quotas.
7. Marketplace manager creates one or more scoped tokens for that publisher.
8. Producer receives a token out-of-band.
9. Producer stores the token as a publisher profile credential.
10. Producer publishes typed packages through the authority.
11. Marketplace manager can revoke tokens, rotate tokens, reduce quota, or
   disable the publisher.

Important rule:

> A producer cannot self-appoint. The marketplace manager creates the publisher
> and controls the token.
> Publishing must authenticate with a publisher token even when the user is
> superadmin. The publish request must not be able to choose `publisher_id`
> directly; publisher identity is derived from the authenticated token.

## 15. Publisher Setup UX

The producer setup flow should optimize for one-time configuration.

Marketplace manager UX:

1. Open Home > Marketplace > Manage This Platform-Owned Marketplace.
2. Create a publisher identity.
3. Set publisher limits, permissions, and expiry policy.
4. Create a scoped publisher token.
5. Copy the token once or export a one-time publisher profile.

Manager boundary rule:

> Publisher creation, publisher quota edits, token creation/revocation, source
> registration, source visibility, and marketplace service enablement belong to
> platform Home. They must not appear in the project Marketplace UI.

Publisher UX:

1. Receive a publisher token from the marketplace manager.
2. Open the project Marketplace or a source-object publish action.
3. Select a saved publisher profile when profile storage exists, or paste the
   one-time publisher token in the publish dialog for the current shippable
   version.
4. Zebflow validates the token and scopes during publish.
5. Future profile-based publish actions reuse the saved publisher profile after
   profile storage exists.

Formal rule:

> The publisher token should be configured once, then reused by publish flows.
> Authors should not paste tokens every time they publish a package.

Current shippable rule before saved publisher profiles exist:

- the publish dialog may accept a one-time pasted publisher token
- the token can also be supplied as a `Bearer` token by API clients
- the backend must authenticate `marketplace:publish` and derive publisher
  identity from the token
- the project `producer_enabled` flag must not grant or deny publish authority;
  token scope and project capability decide the result
- any request-body `publisher_id` must be ignored or rejected for publish

Publisher profile fields:

- `marketplace_url`
- `publisher_id`
- `token_secret_ref`
- `default_visibility`
- `default_package_prefix`
- `last_validated_at`
- `last_validation_status`

Security rule:

> The raw token must never be embedded into a package artifact, README, manifest,
> screenshot, or exported project bundle.

Publisher profile access rule:

> A saved publisher profile is a privileged credential. Project write access
> alone must not automatically grant permission to use it.

Publisher profile use must require:

- `marketplace.publish` project capability, or
- explicit ACL on that publisher profile, or
- marketplace manager capability

Publisher profile storage rule:

> Saved publisher tokens must be stored through the credential/secret system and
> referenced by `token_secret_ref`. They must not be stored in project source.

## 16. Publish From Context UX

Publishing should start where the work already exists.

Preferred entry points:

- file editor action menu
- folder action menu
- pipeline editor action menu
- template editor action menu
- Project Studio Marketplace page

Example flow for `calculator.ts`:

1. User opens `calculator.ts`.
2. User selects "Publish to Marketplace" from the file action menu.
3. Zebflow detects package kind as script/function.
4. User selects the marketplace publisher profile.
5. Zebflow previews dependencies and package tree.
6. User writes or selects `README.md`.
7. User selects screenshots or demo assets from project files.
8. User fills metadata.
9. Zebflow validates quota, excluded files, and package safety.
10. User publishes.

Publish wizard sections:

- source and destination
- package type
- dependency tree
- metadata
- README
- screenshots and media
- database schema effects
- security and quota summary
- publish confirmation

Publish authorization rule:

> Publish is authorized against the selected publisher profile, not only against
> the source project.

Publish must verify:

- target marketplace service is enabled when publishing to the platform-owned
  authority
- selected publisher exists and is enabled
- selected publisher permits publish/update
- selected token or local manager session is allowed to act as that publisher
- package identity belongs to that authority
- updating an existing package is allowed for that publisher

Dependency preview rule:

> The wizard must show the dependency tree before publish, including files that
> Zebflow will include automatically.

Media selection rule:

> The author may select screenshots from anywhere in the project. The published
> artifact stores them under the marketplace service root:
> `services/marketplace-default/packages/{package_id}/versions/{version}/media/`.

Path conflict rule:

> Publish must show normalized artifact paths and reject duplicate paths after
> normalization.

Metadata fields:

- package id
- version
- title
- summary
- tags
- categories
- visibility
- license
- homepage
- support URL
- changelog text or file

Open design question:

- decide whether script/function packages should use package kind `script`,
  `function`, or a more explicit `typescript_function`

## 17. Consumer Flow

Formal consumer flow:

1. User opens Home > Marketplace to explore apps from configured marketplace
   sources, or opens Project Marketplace to install packages into the current
   project.
2. Platform superadmin configures marketplace sources in Home > Marketplace.
3. Normal users browse only public sources and any private sources explicitly
   available to them through platform policy.
4. Zebflow validates remote source URLs against egress policy.
5. User installs a selected package/version into the project.
6. Installed content becomes editable project source.

Project boundary rule:

> Project Marketplace is a client surface. It must not create marketplace
> sources, publishers, tokens, or enable the platform marketplace service.

Default source rule:

> Project Marketplace should seed the default `zebflow-com` source pointing to
> `https://market.zebflow.com/api`. The page must still open as a consumer
> surface even when this platform's own marketplace service is disabled.

Consumer trust rule:

> Installed marketplace content should be treated like imported source code.
> Review it before activating pipelines, deploying templates, or granting
> credentials.

## 18. Consumer Install UX

Consumer UX should be different for apps, folders, pipelines, templates, and
scripts.

Primary consumer surfaces:

- Home Marketplace for apps and full projects
- pipeline editor Add menu for pipeline packages
- file editor Add or Clone menu for scripts and templates
- Project Studio Marketplace for full browsing and management

Package detail should show:

- title
- publisher
- package id and version
- description
- README
- screenshots
- tags and categories
- install modes
- dependency tree
- database schema effects
- exposed app route when applicable
- security warnings and required capabilities

Formal rule:

> The install action must match the package kind. A user should not need to
> understand artifact internals to choose the right action.

Install safety rule:

> Install must never write outside the selected project workspace or selected
> install root.

Install mode matrix:

| Package kind | Primary install mode | Secondary install mode | DB schema default |
| --- | --- | --- | --- |
| project | new project | fork as new project | allowed with review |
| app project | new project then run | fork as new project | allowed with review |
| folder | selected folder in current project | new folder under current project | disallowed by default |
| pipeline | add to current project pipelines | add under chosen namespace | disallowed by default |
| template | add to current project templates | add under chosen namespace | disallowed by default |
| script/function | add to selected folder or registry | add with dependencies | disallowed by default |

Project and app packages:

- should install as a new project by default
- should not silently merge into an existing project
- may expose "install into current project" only as an advanced flow after a
  conflict and schema review
- must preserve source marketplace service instance identity in installed
  metadata

Folder packages:

- should install under a user-selected folder
- must preview path conflicts
- must not create or alter database schema by default
- must reject paths outside the selected install folder

Pipeline, template, and script packages:

- should install into the current project
- should preview imported dependencies
- should require explicit conflict resolution
- should not auto-activate privileged runtime behavior
- must install into a marketplace namespace unless the user explicitly selects
  another destination

Current Project Studio Add+ mode:

- `add_to_current_project` installs the package into the current project
  workspace namespace
- `clone_as_folder` clones the package as a folder inside the current project
  workspace namespace
- app/project `clone_as_new_project` remains a Home Marketplace flow until the
  project-studio app install review flow is explicit

## 19. Database Schema Install UX

Some app packages may need database schema creation during installation.

Formal rule:

> Database schema effects are allowed for app/project packages after review.
> They are not implicit side effects of installing smaller reusable packs.

Recommended schema contract:

- schema changes are declared in the package manifest
- install UI shows a dry-run summary
- table creation, index creation, and seed data are separate categories
- runtime data is never packaged
- secret values are never packaged
- seed data is disabled by default unless the package is explicitly a demo app
- destructive schema operations are rejected by default
- schema initialization is idempotent or versioned
- schema changes run under the installing user's project permissions

Schema install limits:

- project/app package: schema init allowed after review
- folder package: schema init disallowed by default
- pipeline package: schema init disallowed by default
- template package: schema init disallowed by default
- script/function package: schema init disallowed by default

Current open design:

- whether to support a dedicated schema-only package kind
- whether schema changes should generate migrations or direct initialization
- whether current-project app install can ever run schema init, or whether schema
  init must only run for new projects

Formal default:

> Until the schema migration model is explicit, marketplace install must not run
> destructive database operations.

## 20. Installable Apps And `zebflow run`

Marketplace can distribute reusable packs and installable app projects.

These are related but not identical.

Reusable pack:

- installed into an existing project
- writes files under an install root
- may register imported pipelines
- does not create a new project
- does not automatically become the project's public app

Installable app project:

- is a project bundle package
- creates a new local project when installed from Home or CLI
- writes the package files into that new project workspace
- can be opened in Studio after install
- can be run as an app when it has a public route

Current app package path:

- platform/Home marketplace install only accepts project bundles
- a remote project bundle is fetched from:

```text
/api/projects/{owner}/{project}/marketplace/remote/assets/{package}/{version}
```

Current CLI app path:

```text
zebflow run <project-or-marketplace-asset-url>
```

Formal meaning:

- if the target is an installed local project, Zebflow serves that project
- if the target is a marketplace asset URL, Zebflow fetches and materializes it
  as a local project first
- Zebflow then chooses a public webhook route and serves it as the app route

App install safety rule:

> Remote project app install writes into a new project by default. It must not
> overwrite an existing project unless the user explicitly chooses a destructive
> replace flow.

Current public route selection:

1. prefer `GET /`
2. otherwise use the first available `GET` webhook route
3. otherwise use the first webhook route
4. fail if the project has no webhook-triggered public route

Current app URL shape:

```text
/wh/{owner}/{project}{public_path}
```

Formal rule:

> "Run as app" is a runtime entry for an installed project bundle. It is not a
> separate application model and it does not bypass marketplace trust review.

Formal rule:

> A marketplace package should only be treated as an app when it is a project
> bundle with an explicit public entry route.

Project metadata can also mark a project as app-like through marketplace
distribution config:

- `distribution.marketplace.as_app`
- `distribution.marketplace.entry_url`

When `as_app` is true and `entry_url` is set, Home can show an "open app" entry
for the installed project.

Open design questions:

- make app package intent explicit in the package manifest instead of inferring
  only from `project_bundle`
- require an explicit `entry_url` in project bundle metadata for app packages
- show package README/manifest before first run
- add a "Run installed app" button after marketplace install
- decide whether installed app pipelines should auto-activate or require a
  review step

## 21. Management RBAC

Marketplace has two RBAC layers:

- superadmin/platform RBAC for management routes
- project capability RBAC for client publish/install routes
- marketplace token scopes for remote producer/consumer API calls

Current management route gate:

- manager APIs require superadmin platform authority
- project client APIs do not create publishers, tokens, or sources
- publish APIs require `marketplace:publish` tokens for the current project

Current token scope gate:

- `marketplace:read` for remote read
- `marketplace:publish` for remote publish
- `marketplace:manage` reserved for management-grade token flows

Formal target:

- add explicit project capabilities:
  - `marketplace.read`
  - `marketplace.install`
  - `marketplace.manage`
  - `marketplace.publish`

Until that exists, marketplace manager access should remain conservative.

## 22. Package Artifact Contract

A marketplace artifact should be deterministic enough to inspect, hash, cache,
and reinstall.

Formal artifact fields:

- `schema`
- `asset_kind`
- `source_type`
- `source_owner`
- `source_project`
- `source_ref`
- publisher attribution fields
- package title and description
- file entries

Formal file entry rules:

- every entry must have a normalized relative path
- entries must not escape the artifact root
- entries must not include secrets or runtime data
- install must preserve path safety checks
- artifact paths must be validated before storage
- artifact paths must be validated again before install
- duplicate paths after normalization must be rejected
- artifact bytes must match the manifest hash before install

Recommended artifact layout:

```text
manifest.json
content/
  ...
marketplace/
  README.md
  assets/
    ...
```

Formal rule:

> Authoring paths may be flexible, but artifact paths must be stable.

Required manifest fields:

- authority owner/project
- package id
- version
- publisher id
- package kind
- artifact schema version
- file manifest with path, kind, size, hash, and reason
- media manifest with path, type, size, hash, dimensions, and role
- install manifest
- schema-effects manifest
- created timestamp

Integrity rule:

> Package metadata is not trusted by itself. Install must verify artifact hashes
> and path constraints before writing files.

Projection rule:

> Internal manifests may contain authority and source coordinates for integrity,
> audit, and collision prevention. Public marketplace APIs must project those
> manifests into privacy-preserving responses that hide owner/project slugs.

## 23. What Is Not Marketplace

Marketplace is not:

- a secret transfer mechanism
- a live database migration mechanism
- a backup system
- an unauthenticated project sharing system
- a way to make every project public
- a replacement for Git history
- a runtime deployment policy

## 24. Current Implementation Checklist

Already present:

- office-hosted `marketplace-default` service authority for public marketplace
  browsing and publishing storage
- disabled-by-default marketplace service management
- superadmin-only marketplace service activation policy
- password confirmation for marketplace service activation
- publisher records
- scoped marketplace tokens
- token revoke flow
- hashed token storage
- token expiry and last-used tracking
- project and platform repository sources
- remote source egress blocking
- typed publish sources for project, folder, pipeline, and template packages
- local package listing and install flows
- platform/Home install flow for remote project bundles
- platform/Home Marketplace split into app exploration and superadmin-only
  owned-marketplace management
- superadmin-only platform marketplace source registration with public/private
  source visibility
- project Marketplace cleaned up to install/publish client behavior only
- project Marketplace default `https://market.zebflow.com/api` source seeding
- project Marketplace publish-source, preview, my-package, and publish routes
  are token/client scoped instead of gated by `producer_enabled`
- Project Studio Add+ supports explicit `add_to_current_project` and
  `clone_as_folder` marketplace modes
- `zebflow run <project-or-marketplace-asset-url>` app runtime entry
- public route selection for installed/runnable projects
- default public marketplace base URL preset

Needs review before shipping as complete:

- dedicated marketplace project capabilities instead of `PipelinesWrite`
- first-class token rotation UI
- authority-scoped package and version lookup
- publisher profile storage for one-time token setup
- publisher profile ACL and publish authorization
- public marketplace alias/proxy that hides owner/project slugs
- public marketplace API projection that strips authority/source/user slugs
- publish-from-context actions in file, folder, pipeline, and template editors
- package README and media asset ingestion
- image validation, metadata stripping, and marketplace-owned media serving
- sanitized Markdown renderer and marketplace detail CSP
- artifact path validation before publish, storage, and install
- artifact size limiting before remote JSON parsing
- exact `unlisted` visibility behavior
- script package support level in UI and service
- explicit app package manifest contract
- explicit `entry_url` requirement for app packages
- install-to-run review flow for project bundles
- complete install mode selection per package kind
- database schema dry-run and review flow
- package manifest documentation shown in Marketplace detail UI
- stronger package dependency closure tests
- audit log coverage for producer enablement, publisher CRUD, token CRUD, and
  package publish/install

## 25. Stable Mental Model

1. A marketplace is a platform service instance hosted by one office.
2. A project does not become a marketplace by default.
3. Only superadmin can enable and manage the platform-owned marketplace service.
4. Marketplace service enablement requires explicit confirmation.
5. Marketplace manager creates publisher identities.
6. Publisher identity is public and stable.
7. Tokens are private, scoped, revocable credentials.
8. Saved publisher profiles are privileged credentials with their own access
   rules.
9. Producers publish typed packages through a curated publisher identity.
10. Package and version identity is scoped to one authority.
11. Consumers browse configured repository sources.
12. Public marketplace consumers see aliases and curated publisher identity, not
    owner/project slugs.
13. Remote marketplace sources must pass egress policy.
14. Publishers are limited by marketplace-manager quota settings.
15. Package media is ingested, validated, and served by the marketplace.
16. Publish UX starts from the source object and normalizes the package artifact.
17. Install UX follows package kind and shows dependency/schema effects.
18. Project bundle packages can be installed and run as apps when they expose a
    public route.
19. Package install imports source into a project; it does not import trust.
20. Marketplace never transfers secrets, runtime data, logs, caches, or `.git/`.
