# Zebflow Hub Formal Guide

Draft status: review draft.

This guide defines the intended stable model for the Zebflow hub.

The hub should be:

- closed by default
- platform-scoped and office-hosted, with superadmin-only enablement for the
  shippable version
- curated
- typed
- dependency-aware
- safe to consume without becoming a producer
- explicit about publisher identity, token scope, and trust boundaries

The hub should not become a hidden public project browser, an arbitrary
file dump, or a way to bypass project permissions.

## 1. First Principles

Hub exists to move reusable Zebflow work between projects and offices.

It is not the same thing as:

- project visibility
- Git sync
- backup and restore
- runtime state migration
- public anonymous file hosting

Formal principle:

> Hub distributes explicit packages through an explicit hub
> authority. It does not expose projects automatically.

## 2. Hub Authority

A hub authority is a platform service instance hosted by one Zebflow
office.

Public authority URL:

```text
https://hub.zebflow.com/api
```

The public URL is an opaque authority alias. Consumers should not need to know
the internal owner/project slugs used by the host instance.

Primary public package routes:

```text
GET    /api/hub/remote/assets
POST   /api/hub/remote/assets
DELETE /api/hub/remote/assets/{package_id}
GET    /api/hub/remote/assets/{package_id}/{version}
GET    /api/hub/remote/assets/{package_id}/{version}/artifact
GET    /api/hub/remote/assets/{package_id}/media/{media_name}
```

Project Studio also has project-scoped consumer and publish routes under:

```text
/api/projects/{owner}/{project}/hub/...
```

Those routes are a project UI/API surface. They are not the public Hub
authority identity.

Formal authority model:

```text
service_instance_id: hub-default
service_kind: hub
display_label: Hub
host_office_id: {office_id}
state_office_id: {office_id}
public_base_url: https://hub.zebflow.com/api
enabled: true
status: online
placement_generation: 1
```

This means the hub service is platform-scoped and office-hosted. It is
not a normal project.

Formal functions:

```text
service_host(hub-default) = {office_id}
state_host(hub-default) = {office_id}
service_manager(hub-default) = root({office_id})
```

Base rule:

> In the base model, hub runtime and hub state live in the same
> host office. Controller governance must not move the hub database into
> the controller.

The platform Home surface can browse hub sources, but Home is not itself
the authority. Home is a consumer surface.

Formal rule:

- one hub service instance belongs to one host office
- hub service is not a project
- hub service enablement is platform-level
- hub service management is disabled by default and superadmin-only
- project-side publishing is token-scoped; a project becomes a producer only
  when it has a valid publisher token/profile for that hub
- package identity is scoped to one hub service instance
- public consumers use hub aliases, not internal owner/project
  coordinates

## Hub Access Grants

Hub access is split into three separate records:

- Hub service: the platform service instance and public/API base URL
- Hub source/access: the registered remote Hub URL plus any read token
- Hub grant: the explicit assignment from a platform-owned source to projects

Registering a Hub service does not grant project access.
Creating a platform Hub source does not grant project access.
Project access exists only when one of these is true:

- the project owns its own Hub source
- a platform Hub source has a grant for `all_projects`
- a platform Hub source has a grant for the selected project

This is intentional. A platform administrator may own many Hub sources, including
private client or internal-team Hubs, without exposing all of them to every
project. The ergonomic internal-reusables case is still explicit: create one
platform source, create one `all_projects` read grant, and every project can
browse/install from that source without manual project setup.

Grant scopes:

- `all_projects`
- `selected_project`

Grant capabilities:

- `can_read`: browse/install from the source
- `can_publish`: allowed to publish through that access
- `can_manage`: allowed to manage that access

Asset visibility remains separate from access. A project must first be able to
reach the Hub source, then the Hub service decides whether that token/account may
see a public/private package.

Formal package identity:

```text
{service_instance_id}/{package_id}
```

Formal version identity:

```text
{service_instance_id}/{package_id}/{version}
```

Formal rule:

> `package_id` is only unique inside one hub authority. It must never be
> used as a global lookup key by itself.

Implementation rule:

- package reads must include service instance id
- package writes must include service instance id
- version reads must include service instance id and package id
- install flows must preserve source hub identity
- imported remote packages must not overwrite unrelated hub packages

Office placement rule:

> Hub is embodied in its host office. The hub operational
> database, package artifacts, media assets, token hashes, quotas, and local
> service audit records live under the host office while that office hosts the
> hub service.

State ownership rule:

> `state_office_id` must equal `host_office_id` for hub v1. A controller
> may keep inventory and replicated summaries, but those summaries are not the
> source of truth.

Controller rule:

> A controller may govern hub placement and management, but it does not
> become the owner of the hub runtime state unless it is also the
> hub host office.

Privacy rule:

> `{owner}/{project}` is an internal authority coordinate. Public hub
> consumers should only see an opaque hub base URL, repository title,
> package identity, and curated publisher identity.

Alias rule:

> A public hub proxy such as `https://hub.zebflow.com/api` maps to one
> internal authority without exposing its owner or project slug in the consumer
> setup flow.

Public consumer metadata must not expose:

- authority owner slug
- authority project slug
- source owner slug
- source project slug
- platform username behind a publisher

Public consumer metadata may expose:

- hub alias
- repository title
- package id
- package version
- package title and description
- curated `publisher_id`
- curated publisher display name
- curated publisher URL

Public API projection rule:

> Remote/public hub list, detail, search, and install-preview APIs must
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

## 3. Who Can Become A Hub

Current policy:

- only superadmin can enable and manage the platform hub service
- normal user projects can consume hub packages
- normal user projects can publish only through publisher tokens/profiles
- normal user projects do not become hub service hosts by accident

Current enablement mechanism:

- the current platform session must be superadmin
- the user must confirm their password
- the user must select a host office
- the user must set or confirm the public hub base URL
- the hub service placement row is enabled only after those checks pass

Ownership decision:

The hub service belongs to the selected host office for runtime and
state. It belongs to the platform/controller for governance while the office is
federated.

The product policy for the shippable version should be:

> Only superadmin can enable the hub service and assign it to an office.

This means `service_instance_id` is the authority identity. Project
owner/project slugs are not part of public hub identity.

Formal rule:

> Hub service activation is a privileged exposure event. It must require
> explicit confirmation and must not happen as a side effect of project creation,
> package installation, or repository browsing.

## 4. Closed-By-Default Route Model

Hub has two broad route classes:

- consumer routes
- producer routes

Consumer routes allow browsing and installing from configured sources.

Producer routes allow the authority to:

- list publish sources
- preview packages
- publish packages
- manage publishers
- create and revoke tokens
- serve remote hub APIs

Formal route rule:

- management routes must check superadmin authority and hub service
  state where applicable
- project publish-source, preview, and install routes must check project
  capability
- publish routes must require `hub:publish` tokens and derive publisher
  identity from the token
- delete package routes require a hub bearer token:
  - `hub:publish` may delete packages owned by that publisher
  - `hub:manage` may delete any package in the authority
- remote producer APIs must require hub bearer tokens where appropriate
- a disabled authority must not leak publish sources, token lists, publisher lists,
  or private packages

Current project capability used by manager routes:

- hub manager operations currently require project write-level authority
  through `PipelinesWrite`

Required design:

- introduce a dedicated `HubManage` project capability instead of
  reusing `PipelinesWrite`

Formal route capability target:

- consumer browse: `hub.read`
- consumer install: `hub.install`
- publisher profile use: `hub.publish`
- publisher and token management: `hub.manage`
- producer enablement: platform superadmin plus password confirmation

## 5. Consumer And Producer Roles

Hub usage splits into two roles.

Consumer means:

- configure hub repository sources
- browse visible packages
- install a package into a project
- use a read token when a remote source requires one

Producer means:

- operate one enabled hub authority
- create publisher identities
- issue publisher tokens
- publish packages
- revoke tokens
- disable publishers

Formal rule:

> A project can be a consumer without being a producer.

Formal rule:

> A producer is curated through hub management. A random project owner or
> package author does not automatically receive producer power.

## 6. Publisher Identity

Hub identity has three layers:

- platform user
- hub authority
- publisher identity

Platform user:

- logs into Zebflow
- has project capabilities
- performs hub manager actions
- appears in audit trails

Hub authority:

- is the office-hosted hub service
- owns publisher records, package records, package versions, and tokens

Publisher identity:

- is the public identity shown on packages
- is stable across token rotation
- is curated by the hub manager
- is not the platform user slug
- is not the source project slug

Formal rule:

> A platform user is not automatically a public publisher.

Formal rule:

> A publisher is created by a hub manager inside one enabled authority.

Public attribution rule:

> Public hub consumers see the curated publisher identity. They must not
> see the backing platform owner/project slugs used to host the hub.

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

## 7. Hub Tokens

Hub tokens are credentials issued by the hub manager for one
publisher inside one authority.

A token is not the publisher identity. A token acts as that publisher.

Current token scopes:

- `hub:read`
- `hub:publish`
- `hub:manage`

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

1. Hub manager creates or updates a publisher.
2. Hub manager creates a token for that publisher with explicit scopes.
3. Hub manager gives the token to the producer.
4. Producer uses the token for remote publish or read flows.
5. Hub manager revokes the token when it is leaked, expired, rotated, or
   no longer needed.
6. Hub manager creates a replacement token if the publisher should keep
   publishing.

Current refresh mechanism:

- token refresh is modeled as revoke old token plus create a new token
- publisher identity remains unchanged

Open design question:

- expose this as a first-class "Rotate token" UI action that performs revoke and
  create in one workflow

## 8. Publisher Limits And Quotas

Publisher limits belong to the hub authority's publisher settings.

Formal rule:

> The hub manager decides how much a publisher may publish. The
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

> A hub authority may keep old versions, but retention must be bounded
> by `max_versions_per_package` or an explicit authority retention policy.

## 9. Content Types And Asset Kinds

Hub must distribute typed content, not arbitrary folders.

Formal source types:

- `pipeline_with_dependencies`
- `template_with_dependencies`
- `folder_files`
- `project_files`

Canonical asset kinds:

- `pipeline_bundle`
- `template_bundle`
- `folder_bundle`
- `project_bundle`
- `node_bundle`

Source-to-asset mapping:

| Source type | Asset kind |
| --- | --- |
| `pipeline_with_dependencies` | `pipeline_bundle` |
| `template_with_dependencies` | `template_bundle` |
| `folder_files` | `folder_bundle` |
| `project_files` | `project_bundle` |
| node package source | `node_bundle` |

Formal rule:

> Asset kind is a closed contract. Publish/import code must reject unknown asset
> kinds instead of storing arbitrary strings.

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

Current invalid kind behavior:

```text
HUB_ASSET_KIND_INVALID
```

## 9.1 Package Payload Contract

Public remote publish uses:

```http
POST /api/hub/remote/assets
Authorization: Bearer <hub-token>
Content-Type: application/json
```

Minimal package payload:

```json
{
  "package_id": "my-pack",
  "version": "0.1.0",
  "title": "My Pack",
  "description": "Reusable Zebflow asset.",
  "visibility": "public",
  "tags": ["demo"],
  "source_owner": "external",
  "source_project": "external",
  "source_kind": "pipeline_with_dependencies",
  "source_ref": "pipelines/demo.zf.json",
  "artifact": {
    "schema": "zebflow.asset-pack.v1",
    "asset_kind": "pipeline_bundle",
    "source_type": "pipeline_with_dependencies",
    "source_owner": "external",
    "source_project": "external",
    "source_ref": "pipelines/demo.zf.json",
    "publisher_id": "zebflow-official",
    "publisher_display_name": "Zebflow Official",
    "publisher_url": "",
    "publisher_email": "",
    "title": "My Pack",
    "description": "Reusable Zebflow asset.",
    "files": [
      {
        "rel_path": "pipelines/demo.zf.json",
        "kind": "json",
        "size_bytes": 2,
        "reason": "primary",
        "encoding": "text",
        "content": "{}"
      }
    ]
  }
}
```

Package update rule:

> Updating a package means publishing the same `package_id` with a new
> `version`. The package metadata row is updated; historical versions remain
> available until retention policy or package deletion removes them.

Current delete rule:

> Package delete removes the package and all versions. Version-level delete is
> not part of the current contract.

## 10. Dependency Closure

Hub packages should be useful after installation.

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

Hub package artifacts must exclude:

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

Hub packages need enough media to let consumers inspect what they are
installing, but hub media must not become arbitrary public file hosting.

Gallery principle:

> Hub browsing should feel like a disciplined software package registry with a
> Steam-like presentation layer: cover image, screenshots, optional demo video,
> tags, version, summary, and publisher identity. The install contract remains
> strict and Cargo-like.

The gallery contract is optional. A package without images or video remains a
valid package.

Current gallery card fields:

- `package_id`
- `title`
- `description`
- `asset_kind`
- `latest_version`
- `tags`
- `publisher_id`
- `publisher_display_name`
- `publisher_url`
- `image_url`

Publisher link rule:

> The publisher URL is the only general external link shown directly on package
> cards. Package-specific docs should live in markdown/package description
> content rather than adding arbitrary card links.

Formal package media layout:

```text
hub/
  README.md
  assets/
    icon.png
    screenshot-1.png
    demo.gif
```

Formal rule:

> A published artifact stores package description and media under the
> hub service package version root:
> `services/hub-default/packages/{package_id}/versions/{version}/`.
> The authoring location inside the source project may be more flexible.

Current implementation stores the installable payload as:

```text
{data_root}/services/hub-default/packages/{package_id}/versions/{version}/artifact.json
```

Hub media is presentation metadata, not the install payload. An image inside
`artifact.files` is installable, but it is not automatically the Hub card
preview image. Preview images must be registered as package media/cover so they
can be served through:

```text
/api/hub/remote/assets/{package_id}/media/{media_name}
```

Implemented media structure:

```json
{
  "image_url": "/api/hub/remote/assets/my-pack/media/cover.webp",
  "media": [
    {
      "name": "cover.webp",
      "role": "cover",
      "content_type": "image/webp",
      "size_bytes": 120000,
      "sha256": "...",
      "encoding": "base64",
      "content": "..."
    }
  ]
}
```

Formal gallery metadata target:

```json
{
  "gallery": {
    "cover": {
      "kind": "image",
      "media_name": "cover.webp",
      "alt": "Fleet Pulse dashboard"
    },
    "items": [
      {
        "kind": "image",
        "media_name": "screenshot-map.webp",
        "alt": "Live vehicle map"
      },
      {
        "kind": "youtube",
        "url": "https://www.youtube.com/watch?v=...",
        "title": "Demo video"
      }
    ]
  }
}
```

Canonical gallery item kinds:

- `image`
- `youtube`

Gallery item rules:

- `image` items must refer to Hub-controlled media by `media_name`
- `youtube` items may use YouTube URLs only
- arbitrary iframe/embed HTML is not allowed
- arbitrary external image URLs are not allowed as rendered gallery media
- unknown gallery item kinds are rejected
- cover image is optional
- video is optional

Markdown description target:

```json
{
  "summary": "Short one-line package summary.",
  "description_md": "## Overview\nLonger sanitized markdown description..."
}
```

Current implementation note:

> Current code has `description`, `image_url`, and `media[]`. It does not yet
> have first-class `gallery`, `summary`, `description_md`, or YouTube metadata.

Package detail documentation should describe:

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
- serve ingested media from hub-controlled immutable URLs
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
> replaced with unsafe content. Hub detail pages should render reviewed
> hub-owned media.

README rendering rule:

> Hub README content is sanitized Markdown. Raw HTML is not part of the
> v1 rendering contract.

README sanitizer must:

- reject or strip `<script>`, `<style>`, `<iframe>`, event handlers, and raw HTML
- reject `javascript:`, `data:`, `file:`, and private-network URL schemes
- rewrite relative image references to hub-owned media URLs
- render external HTTPS image references as links unless explicitly reviewed
- add safe link attributes for external links
- enforce a Content Security Policy on hub detail pages

Authoring examples:

```text
src/
  calculator.ts
assets/
  screenshot1.png
  screenshot2.png
```

The user may publish `src/calculator.ts` and select `assets/screenshot1.png`
and `assets/screenshot2.png` as hub screenshots. The resulting artifact
is normalized to:

```text
content/
  src/
    calculator.ts
hub/
  README.md
  assets/
    screenshot1.png
    screenshot2.png
```

Optional advanced convention:

```text
tools/calculator/
  calculator.ts
  hub/
    README.md
    assets/
      screenshot1.png
      screenshot2.png
```

The publish wizard may auto-detect the folder-level `hub/` directory,
but manual selection of README and media assets should always be available.

## 12. Visibility

Package visibility and project visibility are different.

Current package visibility values:

- `public`
- `private`
- `unlisted`

Formal rule:

> Home project visibility does not imply hub package visibility.

Formal rule:

> Hub package visibility does not imply direct access to the source
> project.

Read behavior:

- public packages can be listed by remote consumers
- private packages require an authenticated hub token or local authority
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

- `authority_alias` is the preferred public reference for hub sources
- `remote_owner` and `remote_project` are internal/private deployment fallback
  fields
- public hosted hub presets must leave `remote_owner` and
  `remote_project` empty

Formal URL rule:

> Remote hub sources must be public HTTP(S) URLs.

Blocked targets include:

- localhost
- loopback IPs
- private RFC1918 networks
- link-local networks
- carrier-grade NAT
- multicast and unspecified addresses

Formal reason:

Hub remote fetch is an outbound network action. It must not become SSRF
against local infrastructure, cloud metadata, office-private services, or
controller internals.

Direct authority URL form:

```text
https://host/api/hub
```

This form points at an ownerless public Hub API. Project-coordinate Hub routes
are internal project UI/API surfaces, not the public authority shape.

Proxy/base URL form:

```text
https://hub.example.com/api
```

When using a registered hub alias, the repository must not require
`remote_owner` or `remote_project` from the user. The proxy resolves the alias
to the internal authority.

When using a generic API base that is not a registered alias, the repository may
require `remote_owner` and `remote_project` so the client can construct the
direct authority route. This is an advanced/private-deployment mode, not the
default public hub UX.

Consumer privacy rule:

> Outside a bare internal-network direct URL, there should be no consumer-facing
> flow that reveals the authority owner slug, authority project slug, source
> owner slug, or source project slug.

Remote fetch rule:

> Hub remote fetch must validate the final request URL and every
> redirect target against egress policy.

Remote response rule:

> Remote artifact responses must be size-limited before full JSON parsing.

## 14. Producer Flow

Formal producer flow:

1. Superadmin opens platform-level Hub management.
2. Superadmin enables the hub service by selecting the hub host
   office and confirming their password.
3. Controller creates or updates `PlatformServiceInstance`.
4. Selected office provisions hub service runtime and state.
5. Hub manager creates a publisher identity.
6. Hub manager sets publisher permissions and quotas.
7. Hub manager creates one or more scoped tokens for that publisher.
8. Producer receives a token out-of-band.
9. Producer stores the token as a publisher profile credential.
10. Producer publishes typed packages through the authority.
11. Hub manager can revoke tokens, rotate tokens, reduce quota, or
   disable the publisher.

Important rule:

> A producer cannot self-appoint. The hub manager creates the publisher
> and controls the token.
> Publishing must authenticate with a publisher token even when the user is
> superadmin. The publish request must not be able to choose `publisher_id`
> directly; publisher identity is derived from the authenticated token.

## 15. Publisher Setup UX

The producer setup flow should optimize for one-time configuration.

Hub manager UX:

1. Open Home > Hub > Manage This Platform-Owned Hub.
2. Create a publisher identity.
3. Set publisher limits, permissions, and expiry policy.
4. Create a scoped publisher token.
5. Copy the token once or export a one-time publisher profile.

Manager boundary rule:

> Publisher creation, publisher quota edits, token creation/revocation, source
> registration, source visibility, and hub service enablement belong to
> platform Home. They must not appear in the project Hub UI.

Publisher UX:

1. Receive a publisher token from the hub manager.
2. Open the project Hub or a source-object publish action.
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
- the backend must authenticate `hub:publish` and derive publisher
  identity from the token
- the project `producer_enabled` flag must not grant or deny publish authority;
  token scope and project capability decide the result
- any request-body `publisher_id` must be ignored or rejected for publish

Publisher profile fields:

- `hub_url`
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

- `hub.publish` project capability, or
- explicit ACL on that publisher profile, or
- hub manager capability

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
- Project Studio Hub page

Example flow for `calculator.ts`:

1. User opens `calculator.ts`.
2. User selects "Publish to Hub" from the file action menu.
3. Zebflow detects package kind as script/function.
4. User selects the hub publisher profile.
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

- target hub service is enabled when publishing to the platform-owned
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
> artifact stores them under the hub service root:
> `services/hub-default/packages/{package_id}/versions/{version}/media/`.

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

1. User opens Home > Hub to explore apps from configured hub
   sources, or opens Project Hub to install packages into the current
   project.
2. Platform superadmin configures hub sources in Home > Hub.
3. Normal users browse only public sources and any private sources explicitly
   available to them through platform policy.
4. Zebflow validates remote source URLs against egress policy.
5. User installs a selected package/version into the project.
6. Installed content becomes editable project source.

Project boundary rule:

> Project Hub is a client surface. It must not create hub
> sources, publishers, tokens, or enable the platform hub service.

Default source rule:

> Project Hub should seed the default `zebflow-com` source pointing to
> `https://hub.zebflow.com/api`. The page must still open as a consumer
> surface even when this platform's own hub service is disabled.

Consumer trust rule:

> Installed hub content should be treated like imported source code.
> Review it before activating pipelines, deploying templates, or granting
> credentials.

## 18. Consumer Install UX

Consumer UX should be different for apps, folders, pipelines, templates, and
scripts.

Primary consumer surfaces:

- Home Hub for apps and full projects
- pipeline editor Add menu for pipeline packages
- file editor Add menu for scripts and templates, with Clone as folder only as
  a mode where relevant
- Project Studio Hub for full browsing and management

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
- must preserve source hub service instance identity in installed
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
- must install into a hub namespace unless the user explicitly selects
  another destination

Current Project Studio Add+ mode:

- `add_to_current_project` installs the package into the current project
  workspace namespace
- `clone_as_folder` clones the package as a folder inside the current project
  workspace namespace
- app/project `clone_as_new_project` remains a Home Hub flow until the
  project-studio app install review flow is explicit

## 19. Database Schema Install UX

Some app packages may need database schema creation during installation.

Formal rule:

> Database schema effects are allowed for app/project packages after review.
> They are not implicit side effects of adding smaller reusable packages.

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

> Until the schema migration model is explicit, hub install must not run
> destructive database operations.

## 20. Installable Apps And `zebflow run`

Hub can distribute reusable packages and installable app projects.

These are related but not identical.

Reusable package:

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

- platform/Home hub install only accepts project bundles
- a remote project bundle is fetched from:

```text
/api/hub/remote/assets/{package}/{version}
```

Current CLI app path:

```text
zebflow run <project-or-hub-asset-url>
```

Formal meaning:

- if the target is an installed local project, Zebflow serves that project
- if the target is a hub asset URL, Zebflow fetches and materializes it
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
> separate application model and it does not bypass hub trust review.

Formal rule:

> A hub package should only be treated as an app when it is a project
> bundle with an explicit public entry route.

Project metadata can also mark a project as app-like through hub
distribution config:

- `distribution.hub.as_app`
- `distribution.hub.entry_url`

When `as_app` is true and `entry_url` is set, Home can show an "open app" entry
for the installed project.

Open design questions:

- make app package intent explicit in the package manifest instead of inferring
  only from `project_bundle`
- require an explicit `entry_url` in project bundle metadata for app packages
- show package README/manifest before first run
- add a "Run installed app" button after hub install
- decide whether installed app pipelines should auto-activate or require a
  review step

## 21. Management RBAC

Hub has two RBAC layers:

- superadmin/platform RBAC for management routes
- project capability RBAC for client publish/install routes
- hub token scopes for remote producer/consumer API calls

Current management route gate:

- manager APIs require superadmin platform authority
- project client APIs do not create publishers, tokens, or sources
- publish APIs require `hub:publish` tokens for the current project

Current token scope gate:

- `hub:read` for remote read
- `hub:publish` for remote publish
- `hub:manage` reserved for management-grade token flows

Formal target:

- add explicit project capabilities:
  - `hub.read`
  - `hub.install`
  - `hub.manage`
  - `hub.publish`

Until that exists, hub manager access should remain conservative.

## 22. Package Artifact Contract

A hub artifact should be deterministic enough to inspect, hash, cache,
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

Current artifact layout:

```text
services/hub-default/packages/{package_id}/versions/{version}/artifact.json
```

The artifact JSON contains the manifest and file entries. Media/cover images
are presentation assets and are served through Hub media routes rather than
being inferred from arbitrary install files.

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
> audit, and collision prevention. Public hub APIs must project those
> manifests into privacy-preserving responses that hide owner/project slugs.

## 23. What Is Not Hub

Hub is not:

- a secret transfer mechanism
- a live database migration mechanism
- a backup system
- the entire Add+ system
- an unauthenticated project sharing system
- a way to make every project public
- a replacement for Git history
- a runtime deployment policy

Formal separation:

```text
Hub = distribution.
Backup = preservation.
```

Hub packages are clean, versioned, reusable artifacts. They may move through a
Zebflow Hub, GitHub, GitLab, a local folder, or a zip file, but the same package
contract applies in every transport. Hub packages must not include private
runtime state, credentials, logs, caches, `.git/`, or full production data by
default.

Add+ is a broader project-side import surface. Hub is one Add+ source adapter,
not the whole Add+ product.

Canonical Add+ source adapters:

- `built_in`: offline first-party material embedded in the Zebflow binary, such
  as Zeb React UI components and future official starter packages.
- `hub`: packages from a Zebflow Hub source.
- `git`: packages or source folders fetched from GitHub, GitLab, or another git
  remote.
- `local`: local folders and user-selected package archives.

Zip files are local when the user selects or uploads the archive from their
machine. If a zip is fetched from a URL, the source adapter is the remote system
that provided that URL.

Backup/export is a different system. Backup may include project data, files,
database snapshots, selected logs, runtime state, and credential references or
encrypted secrets when explicitly allowed. Its job is to reproduce or restore a
project, not to distribute reusable assets.

### 23.1 Add and Install

Add+ should behave like Unity or Blender asset import, not npm package
installation. Hub packages follow this same safety model when they are added
through Add+.

- `Add`: canonical Hub action that brings a package into the current project.
- `Add to current project`: write the package into its normal destination.
- `Clone as folder`: Add mode that writes a larger package, folder, starter, or
  workflow bundle as a separate folder.
- `Install`: reserve for app-level runnable packages or full project/app
  templates.

Most Hub package actions are therefore Add. Importing a Hub package must not
silently activate runtime behavior.

Before Add, Zebflow should show a safety review:

- source adapter (`built_in`, `hub`, `git`, or `local`)
- package kind or built-in material kind
- files to be added
- files to be overwritten
- files to be skipped
- pipelines to register
- nodes used
- credentials required
- external URLs and domains that may be contacted
- database schemas, tables, collections, graphs, or indexes touched
- filesystem paths touched
- schedules, webhooks, or public endpoints created
- secrets required but not included
- large files included
- seed or demo data included

Pipeline artifacts should land as draft by default. Activation, schedule
creation, public endpoint exposure, credential binding, schema mutation, and
outbound network access require explicit user confirmation.

Rule:

> Package import brings source into the project. It does not import trust.

Policy invariant:

> Every Add+ source adapter must produce the same review shape before it writes.
> A built-in component, Hub package, Git folder, and local zip may have different
> fetch mechanics, but they must all report source, kind, files added,
> overwrites/skips, executable surfaces, credentials, external URLs, database
> effects, filesystem effects, public endpoints, schedules, large files, seed
> data, warnings, and risk level.

Current Add+ source policy:

- `built_in`: review is generated from embedded first-party files before
  installing into `pipelines/shared/ui`.
- `hub`: review is generated from the resolved package artifact before adding
  files into the project repo.
- `git`: must resolve to a package/folder artifact first, then use the same
  review shape before writing.
- `local`: must unpack or inspect the selected folder/archive into a staging
  area first, then use the same review shape before writing.

## 24. Current Implementation Checklist

Already present:

- office-hosted `hub-default` service authority for public hub
  browsing and publishing storage
- disabled-by-default hub service management
- superadmin-only hub service activation policy
- password confirmation for hub service activation
- publisher records
- scoped hub tokens
- token revoke flow
- hashed token storage
- token expiry and last-used tracking
- project and platform repository sources
- remote source egress blocking
- typed publish sources for project, folder, pipeline, and template packages
- local package listing and install flows
- platform/Home install flow for remote project bundles
- platform/Home Hub split into app exploration and superadmin-only
  owned-hub management
- superadmin-only platform hub source registration with public/private
  source visibility
- project Hub cleaned up to install/publish client behavior only
- project Hub default `https://hub.zebflow.com/api` source seeding
- project Hub publish-source, preview, my-package, and publish routes
  are token/client scoped instead of gated by `producer_enabled`
- Project Studio Add+ supports explicit `add_to_current_project` and
  `clone_as_folder` hub modes
- Project Studio Add+ uses policy review before adding Hub packages and
  built-in UI components
- `zebflow run <project-or-hub-asset-url>` app runtime entry
- public route selection for installed/runnable projects
- default public hub base URL preset

Needs review before shipping as complete:

- dedicated hub project capabilities instead of `PipelinesWrite`
- first-class token rotation UI
- authority-scoped package and version lookup
- publisher profile storage for one-time token setup
- publisher profile ACL and publish authorization
- public hub alias/proxy that hides owner/project slugs
- public hub API projection that strips authority/source/user slugs
- publish-from-context actions in file, folder, pipeline, and template editors
- package README and media asset ingestion
- Git and local zip Add+ source adapters wired to the same policy review gate
- image validation, metadata stripping, and hub-owned media serving
- sanitized Markdown renderer and hub detail CSP
- artifact path validation before publish, storage, and install
- artifact size limiting before remote JSON parsing
- exact `unlisted` visibility behavior
- script package support level in UI and service
- explicit app package manifest contract
- explicit `entry_url` requirement for app packages
- install-to-run review flow for project bundles
- complete install mode selection per package kind
- Add safety review UI
- GitHub, GitLab, local folder, and zip package source adapters
- database schema dry-run and review flow
- package manifest documentation shown in Hub detail UI
- stronger package dependency closure tests
- audit log coverage for producer enablement, publisher CRUD, token CRUD, and
  package publish/install

## 25. Stable Mental Model

1. A hub is a platform service instance hosted by one office.
2. A project does not become a hub by default.
3. Only superadmin can enable and manage the platform-owned hub service.
4. Hub service enablement requires explicit confirmation.
5. Hub manager creates publisher identities.
6. Publisher identity is public and stable.
7. Tokens are private, scoped, revocable credentials.
8. Saved publisher profiles are privileged credentials with their own access
   rules.
9. Producers publish typed packages through a curated publisher identity.
10. Package and version identity is scoped to one authority.
11. Consumers browse configured repository sources.
12. Public hub consumers see aliases and curated publisher identity, not
    owner/project slugs.
13. Remote hub sources must pass egress policy.
14. Publishers are limited by hub-manager quota settings.
15. Package media is ingested, validated, and served by the hub.
16. Publish UX starts from the source object and normalizes the package artifact.
17. Install UX follows package kind and shows dependency/schema effects.
18. Project bundle packages can be installed and run as apps when they expose a
    public route.
19. Package install imports source into a project; it does not import trust.
20. Hub never transfers secrets, runtime data, logs, caches, or `.git/`.
