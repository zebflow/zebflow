# Distribution

Status: **Review**

This document owns how a Zebflow resource leaves one instance and arrives at
another. It is not a registered kind. It is the layer that several kinds share,
written down once so each kind's review knows its role instead of rediscovering
the same questions.

Zebflow's premise is that a project can be created, shared, cloned, and
installed easily. That premise only holds if distribution is one system with one
safety model, rather than several install paths that happen to exist.

## 0. Scope

Distribution happens at two levels, and the same words mean different things at
each. Saying "install" without saying where is the main source of confusion, so
the scope comes first.

| Scope | Who acts | What arrives | Result |
| --- | --- | --- | --- |
| **Platform / office** | an operator or account owner | a whole project | a **new project exists** |
| **Project** | someone working inside one project | a dependency or some content | **that project changes** |

At platform scope, `install` means materialise a project. It creates something
that did not exist:

```text
POST /api/users/{owner}/hub/install     install a Hub project into an account
POST /api/platform/hub/install          install a Hub project as a platform app
zeb install <ref>                       the CLI client for the second of those
```

`zeb install` is the client for the second of those and **does not assume an
instance already exists**. On a machine that has just installed the binary it
creates one at the data root above, signs in with the superadmin password first
boot generates and writes 0600, and then installs. Whether it reaches those
routes over the network or calls them in its own process depends only on whether
a server is already listening: `interface.md` §6 is HTTP when a server owns the
state and direct when none does. Nothing is left running by an install that took
the second path, which is why `run` is still a separate command.

Each of the two HTTP surfaces has a review sibling that reports what the install
would do and does none of it:

```text
POST /api/users/{owner}/hub/install/review
POST /api/platform/hub/install/review
```

They take the same body as the install, including the three consent flags, and
answer with the destinations that would be written, the pipelines that would be
registered and activated, what the install-time SQL does, the safety review, and
whether the package is installable at all. Nothing materialises a project
without going through them: `zeb run` used to take a hub asset URL and write one
directly, which was a second path with no review, and it no longer materialises
anything.

At project scope, `install` means take on a managed dependency. The project
already exists and gains something:

```text
POST /api/projects/{owner}/{project}/nodes/install
POST /api/projects/{owner}/{project}/hub/assets/{id}/{version}/add
```

The two are not variants of one operation. Platform-scope install produces a
project and cannot be undone by an uninstall; project-scope install adds a
tracked dependency that uninstall removes. A rule proven at one scope does not
transfer to the other, and this document says which scope each rule belongs to.

The CLI reflects the same split. `zeb project install <ref>`, with its
`zeb install` alias, is the only platform-scope CLI verb that creates anything;
it takes a Hub package reference and runs the review above. `zeb run
<owner>/<project>` serves a project that is already installed and materialises
nothing. `zeb project remove <owner>/<project>`, aliased `zeb remove`, deletes a
project outright — the paragraph above says platform-scope install "cannot be
undone by an uninstall", and `remove` is not one: it is the deletion of the
whole project, and the command says so before it asks. There is no project-scope
CLI verb today; project-scope distribution, uninstall included, is API and UI
only.

## 0b. Consumption modes

Scope says *where* something arrives. Mode says *why*, and the two are
independent.

| Mode | Command | What the user sees |
| --- | --- | --- |
| **Run** | `zeb run <owner>/<project>` | the app, served at its entry point |
| **Develop** | the Zebflow server | the same project, as editable source |

**These are two ways of opening one installation, not two kinds of install.**
There is no runtime-only distribution and no stripped build. The project on disk
is identical either way, and the mode is only how you open it.

That is the property worth protecting. You run a spreadsheet tool. You want it
pink and there is no setting for it. You open the same installation as source,
change it, and run it again. Nothing was compiled away, so nothing has to be
obtained a second time.

An installed Zebflow app is therefore always inspectable and always editable,
which is the opposite of shipping a binary. The warning that belongs beside it
is the honest one: editing the source of a tool you depend on is exactly as
risky as it sounds, and the system should say so rather than pretend the source
is not there.

### Where it lands

```text
zeb install osgeo-spatial-tool             review it, then materialise it
zeb run superadmin/osgeo-spatial-tool      serve it
```

**Obtaining now costs two commands, not one, and that is a deliberate trade.**
The one-command form existed: `zeb run <hub-asset-url>` fetched an artifact and
wrote it straight into a project. It was the only install path in the system
that ran no review, which §3 forbids of every channel, so it was removed rather
than given a second copy of the review. `install` reviews and creates; `run`
serves. Getting back to one command means teaching `run` to call the same
install path, not teaching it to write files again.

Run mode still needs somewhere to serve from, and now it and `install` agree on
where by construction rather than by both happening to read one variable. The
data root has exactly two cases — `ZEBFLOW_PLATFORM_DATA_DIR` when it is set,
and the OS user-data path otherwise (the per-OS list is `interface.md` §5's).
It is never relative to the working directory. The old default was `.zebflow-platform-data`
beside wherever the shell happened to be, which meant an installed binary
created an instance in whatever folder the user was standing in and a `cd` lost
the projects in it — the stable per-user location this paragraph used to say was
owed. `interface.md` §5 states the rule and one function in `platform::boot`
implements it, so a client and a server mode cannot disagree.

Someone who wants a geospatial selection tool should still obtain one thing and
use it. That Zebflow is underneath is not their concern, and this is the mode
that makes distribution worth having.

### A project needs a declared entry point

Run mode has to answer one question: **what does this project serve?**

Today nothing declares it. `choose_public_app_path` scans every pipeline in the
project, collects webhook triggers, and guesses: a `GET /` if one exists, then
any `GET`, then any trigger at all. That works for a project with one obvious
front door and silently picks something arbitrary for a project without one.

Guessing is acceptable as a fallback. It is not acceptable as the contract,
because it means a project cannot state what it is, and an author cannot control
what a user sees first.

`ProjectConfiguration` is the natural owner: it is frozen, project-scoped, and
already carries `profile`, `runtime`, and `bootstrap`. An entry-point
declaration belongs beside them.

That is a change to a frozen contract, so it needs its versioning rules rather
than an edit in passing. Recorded here as a distribution requirement so the
`ProjectConfiguration` change is made deliberately, and so run mode stops
depending on a heuristic.

## 0a. Command line surface

Zebflow's premise is that obtaining something should be one command, the way a
package manager works. Someone who wants a video pipeline built from ElevenLabs
and Seedance nodes, or a set of office tools, should be able to obtain the whole
thing and have it belong to their instance.

**This section is mostly a proposal, and every line below says which.** Four of
its verbs exist today: `zeb project install` with its `zeb install` alias, which
reviews a Hub project bundle, prints what it would write and what SQL it would
run, asks, and then materialises it over the instance's HTTP API; `zeb project
remove` with its `zeb remove` alias, which deletes a project; `zeb project list`
with its `zeb list` alias; and `zeb run`, which serves an installed project.
Everything marked **not built** is unimplemented, and is written here so the
surface is designed once rather than grown one flag at a time — not so that a
reader can expect to type it.

### Which pattern this follows

| CLI | Shape | Context | What it teaches |
| --- | --- | --- | --- |
| `kubectl` | **verb first** — `kubectl get pods` | kubeconfig contexts, `-n` | Verb-first reads well when every resource supports the same small verb set. Zebflow's do not: a project is installed, a node is installed *into* something, source is added. |
| `docker` | verb-first, then noun groups added later — `docker run` and `docker container run` | daemon context | Both forms now coexist permanently. This is the cost of not deciding early, and the outcome to avoid. |
| `aws` | **noun first**, very flat — `aws s3 cp` | `--profile`, `--region` | A flat service list scales to thousands of commands but gives no help with scope. |
| `gcloud` | **noun first**, deep groups — `gcloud compute instances create` | `gcloud config set project` | Consistent noun-verb nesting, with context set once and reused. Closest to what Zebflow needs. |
| `wrangler` | noun groups plus a few blessed top-level verbs — `wrangler d1 create`, `wrangler deploy` | `wrangler.toml` | Groups for structure, short top-level verbs for the everyday path. |

**Zebflow follows `gcloud` and `wrangler`: noun group first, verb second, with a
small number of blessed top-level verbs.**

That is not a new choice. `zeb project config migrate` and `zeb k8s cluster
init` already have this shape, and `zeb install` is already a blessed top-level
verb. The design below continues an existing pattern rather than introducing
one.

The deciding argument is scope. `project install` and `node install` mean
different things — one creates a project, the other changes one — and putting
the noun first is what makes that visible at the point of typing. Verb-first
would put the ambiguous word first and the disambiguating word second.

### The surface

```text
# Platform scope — creates or destroys a project
zeb project install <ref> [--repo <id>]    materialise a project into this instance
zeb project remove <owner>/<project>       delete a project, its data, and its files
zeb project list                           what this instance holds
zeb project export <kind>                  write a transfer archive          (not built)
zeb run <owner>/<project>                  serve an installed project

# Project scope — changes the current project                     (none of it built)
zeb node install <ref> [--repo <id>]       take on a node bundle dependency
zeb node uninstall <kind>
zeb lib add <ref> [--repo <id>]            resolve an RWE library
zeb hub add <ref> --to <folder>            copy content in as this project's source
zeb hub publish <source> --to <hub>        share outward
```

`project install` creates something; `node install` changes something that
already exists. Reading the noun tells you which, which is the property the
scope split in §0 exists to protect.

`add` and `publish` sit under `hub` rather than at the top level. They were
blessed as top-level verbs by an earlier draft of this section, and
`interface.md` §3 objected that neither says *what* is being added or published
without its noun. That objection is accepted: `hub` is where the routes they
would call already live, and moving them costs nothing because neither is
built.

**Context.** Project-scope commands need to know which project. Context comes
from the client context store, `~/.zebflow/client/context.json`
(`interface.md` §5), and the resolution order is settled: explicit
`--owner` / `--project` flag, then stored context, then error. No command
guesses.

**A reference must name its channel.** `<ref>` alone is ambiguous across the
channels in §2, and the channel is the trust decision, so it is never inferred:

```text
zeb node install acme-tools                     this instance's Hub
zeb node install acme-tools --repo acme-mirror  a named repository on this instance
zeb node install ./acme-tools.json              a local file
```

**`--repo` takes a repository id, not a URL.** An earlier draft of this section
wrote `--repo <url>`. What `zeb install --repo` does is name one of the hub
repositories the receiving instance already has configured, which is what the
transport allows: the command runs over the instance's HTTP API, and handing a
server an arbitrary URL to fetch is a new channel with its own trust decision
rather than a flag. Static repositories are now implemented and this line is
unchanged by that: a static source is *configured*, with an id, and then named
by that id.

It is needed only to **override** the resolution order in §2, not to disambiguate
a failure. Two configured sources carrying one package id resolve to the first
one searched; `--repo` is how a person asks for the other. An earlier draft of
that rule refused rather than resolving, which stopped being tenable the moment
a fresh instance had two official sources.

### Blessed top-level verbs

Three verbs earn a top-level place, because they are the headline: obtaining a
whole project, being rid of one, and seeing what you have should each be one
short command.

```text
zeb install <ref>                 alias for: zeb project install <ref>
zeb remove <owner>/<project>      alias for: zeb project remove <owner>/<project>
zeb list                          alias for: zeb project list
```

Each is an **alias**, not an inference. Bare `install` always means the same
thing regardless of any configured project context, because a command whose
meaning changes with hidden state is the failure §0 exists to prevent.
`interface.md` §4 holds the frozen list; this section blesses no verb that is
not in it.

### Binary name

The long name is `zebflow`; the short name is `zeb`. Shortening is the norm —
Kubernetes ships `kubectl`, Google Cloud ships `gcloud`, Cloudflare ships
`wrangler` — and `zeb install` is the shape people will actually type.

Both names exist. The crate builds two binary targets from one source, so `zeb`
and `zebflow` are the same program under two names rather than two programs, and
installing a product called Zebflow and finding only a `zeb` binary is never
surprising. `zeb` also matches the `zeb/*` library namespace and the `zeb.lock`
file, so the short name is already the project's own vocabulary.

Neither name is written into the program. Every usage line, error, and hint
reads `argv[0]`, so a person who typed `zebflow` is told to run `zebflow` and a
person who typed `zeb` is told to run `zeb`. A binary invoked under some third
name reports the primary one.

## 1. What is distributable

Unless a row says otherwise, these are project-scope acts.

| Resource | Kind that owns its format | Channels | Direction |
| --- | --- | --- | --- |
| Node bundle | `NodeBundle` | Hub asset, remote pack, local file, project transfer | export, publish, install |
| RWE library | `RweLibraryManifest` | any hub serving (`rwe_library` package: local blessed, public, static); direct ingestion (same package, same gates, locked `direct.file` / `direct.npm`); Git planned | install only, today |
| Pipeline | `Pipeline` | Hub asset (`pipeline_bundle`) | export, publish, add |
| RWE source: page, component, script, style | no kind yet | Hub asset (`template_bundle`) | export, publish, add |
| Folder of project files | no kind yet | Hub asset (`folder_bundle`) | export, publish, add |
| Whole project | `ProjectBundle` | Hub asset (`project_bundle`), transfer archive, git remote | export, publish, import, clone; **install** at platform scope |
| Project files only | ZebFS objects | transfer archive (`files`) | export, import |
| UI component | catalog entry, no kind yet | built-in catalog | add only |
| Database schema | `DatabaseSchema` | schema export endpoint | export only |
| Credential **values** | none | **none** | **never distributed** |

The credential row is a rule, not a gap. Credential values stay in the
credential service. A package may declare credential *kinds* it needs; it never
carries a secret.

`template_bundle` carries any RWE source file — a page, a component, a `.ts`
behavior script, or a stylesheet — together with its local imports. It is not
limited to pages, and scripts are not a separate mechanism.

Three rows have no owning kind: RWE source files, folders, and UI catalog
components. They are distributed today with no contract governing their shape.
That is a real gap, recorded here rather than in a kind that does not exist.

## 1a. The verbs

Direction alone is not enough, because two resources can both arrive over the
same channel and mean different things afterwards.

Platform-scope install is described in §0 and behaves differently: it produces
a whole project, which the receiving instance then owns and edits freely. At
project scope `install` means *managed*; at platform scope it means
*materialised and yours*.

| Verb | What it means | Lands in | Editable by the receiver | Recorded in `zeb.lock` | Removal |
| --- | --- | --- | --- | --- | --- |
| **Install** (project scope) | takes on a managed dependency | `data/` | no | yes | uninstall |
| **Install** (platform scope) | a whole project is materialised (§0) | a new project | yes, it is theirs | no — the §7 provenance gap | none; irreversible |
| **Add** | copies content into this project's own source | `repo/` | **yes, it becomes their file** | no | delete the files |
| **Import** | replaces or merges whole project areas | `repo/`, `data/`, `files/` | yes | n/a | destructive; no undo |

The consequence that must reach the user: **added content has no update path.**
The receiver may have edited it, so re-adding would destroy their work. Only
installed content can be updated, because only installed content is still owned
by its publisher.

### Destination

Added content needs a destination the receiver chooses, because it becomes their
source and has to sit where their project is organised. Installed content does
not: it is materialised at a path the platform owns.

| Path | Destination | Default |
| --- | --- | --- |
| Hub add | `target_folder`, chosen by the receiver, inside the project's source root | `{source}/hub/{id}` |
| Hub install, node bundle | not chosen | `data/hub/nodes/{package}` |
| UI catalog add | **not offered** | fixed by the catalog |

A package's paths were produced by the publisher's layout, so `target_folder`
names the destination of its **source** files only. Assets and docs go to the
directories the receiving project declares for them, under the same folder name;
anything else stays inside the package's own folder. A release records the
publisher's layout so the translation is read rather than guessed — see
[`kinds/hub-package/README.md`](./kinds/hub-package/README.md#install-maps-into-the-receiving-projects-layout).

The last row is an inconsistency, not a design. Adding a component is the same
act as adding an RWE source file from the Hub, and it should offer the same
destination choice. It also takes an `overwrite` flag rather than reporting the
collision, so re-adding a component the receiver has edited destroys that work
with no diff and no copy.

Both are corrections owed to the catalog path, recorded here because the rule
belongs to `add`, not to one endpoint.

## 1b. Hub: one format, three servings

A hub is one format: `HubPackage` release documents plus content-addressed
artifacts (sha256). A release moves between all three servings without changing
a byte, and the installer cannot tell the difference. The only differentiator
is how it is served; the catalogue wrapper (how packages are *found*) differs
per serving and is never part of the package.

**Local** — the blessed shelf, read in-process at `services/hub-local/`.
Present in every office's data root unconditionally (same release →
byte-identical) and **immutable**: its only writer is the seed, which runs at
every boot, check-first — it publishes the running release's blessed
`zebflow.*` content through the real publish core as the reserved `zebflow`
publisher, publishing only coordinates absent from the shelf; a coordinate
already present is never rewritten or deleted. No other write path exists —
not superadmin — and no publisher token can be minted for or target it. There
is no blessed retraction, deliberately: a bad blessed version is superseded by
a release update seeding a fixed coordinate beside it. This serving is why an
offline machine — a Raspberry Pi with nothing but the installer — still
installs blessed content. No sharing semantics and no ACL of its own — nothing
to govern when nothing but the seed writes. All publishing by anyone on this
instance, superadmin included, goes to the Public Hub.

**Public** — the Public Hub service's own store, `services/hub-public/`,
served over HTTP. The service is placed: an OfficeTopology
`PlatformServiceInstance` — host office, state owner, `placement_generation`
([`kinds/office-topology/README.md`](./kinds/office-topology/README.md)) — and
`services/hub-public/` exists only in the state-owning office's data root.
Every consumer, other offices and the host's own projects alike, reaches it by
its base URL and locks `hub.public`; the state-owning office may read its own
store in-process instead of looping back over HTTP — provenance is the store,
transport is implementation, the lock says `hub.public` either way. The ONE
publish target and the ONLY
sharing mechanism, with the one ACL model: publisher / token / grant. Exposed
by k8s/nginx → an internet hub like npm. Not exposed → reached by internal URL
(localhost included), and privacy is network topology, not a second permission
system. Admin sugar: superadmin may auto-create publisher accounts and grants
directed at chosen projects.

**Static** — the same content as plain read-only files at any HTTPS URL:
`zebflow-repository.json` ([`HubRepositoryIndex`](./kinds/hub-repository-index/README.md))
plus package documents and artifacts. Publish is committing files. Trust is the
URL the user named plus locked digests; the host (GitHub or anyone) is pure
transport — substituted bytes fail the digest, and the install review refuses
smuggled executables regardless of origin. Official: `github.com/zebflow/hub`.

| Publish target | Route | Who can see it | Trust basis |
| --- | --- | --- | --- |
| Local hub | none — seeded by the release, immutable | everyone on this instance | the release itself |
| Public Hub | `hub/remote/assets/publish` | per its access rules and exposure | publisher token plus a repository grant |

## 2. Channels

A channel is a way bytes move. Each has a different trust story, and that is the
reason they are listed separately rather than treated as one "install".

Channels are finer-grained than §1b's three hub servings, and the two
vocabularies do not compete: **hub asset** is the local serving, **remote pack**
is the public serving, **static repository** is the static serving — and the
other channels (embedded, local file, transfer archive, git remote) move bytes
without any hub at all.

| Channel | Source of bytes | Trust basis | Today |
| --- | --- | --- | --- |
| Embedded | the Zebflow binary | the release itself | the seed; official node bundles |
| Local file | a document the user supplies | the reviewing user | node bundles; RWE libraries |
| Hub asset | the blessed shelf (`services/hub-local/`), read in-process | the release seed + stored digest | what the release seeds: RWE libraries, template sets — the format takes all six asset kinds; the shelf's content is the seed |
| Remote pack | a public hub over HTTP — any instance's, including this one's own | repository grant + artifact digest | all asset kinds; packs, projects |
| Transfer archive | an export file | whoever produced it | project bundle, files |
| Git remote | a git repository | the remote's own access control | project `repo/` |
| Static repository | an HTTPS location serving an index and package documents | the repository URL the user named, plus a locked digest | project bundles, platform scope |

**Embedded is not a channel a user invokes.** It is listed because it is how
official content arrives, and because a resource moving from embedded to
installed is a distribution change even though no bytes travel. It is now
first of all **the seed**: at every boot, check-first, the blessed content the
binary carries — RWE libraries and the UI template sets, as `zebflow.*` — is
published into the local hub through the ordinary publish gates (a coordinate
already present is skipped, per §1b), and what a
project installs afterwards is a hub asset with a lock entry, not embedded
bytes. `embedded` is gone from the lock vocabulary — a lock still saying it is
invalid and is regenerated by ordinary resolution — and the binary's blessed
tree is only the seed, never a serving a lock can name.

### One repository interface

The repository interface fronts the five **package** channels — embedded,
local file, hub local, hub public/remote, static — as five implementations of
one interface, and the format they carry is the same in every case. Transfer
archives and git remotes move whole projects outside it.

A repository answers three questions:

```text
list()                    what packages and versions are available here
fetch(id, version)        give me that HubPackage document
artifact(id, version, d)  give me the bytes that hash to d
```

The third call exists because a package's files are carried inline or referenced
by digest, and a reference names no location: the channel supplies it. A
repository that serves documents and not their artifacts can only serve packages
small enough to carry everything.

Everything else — publisher identity, access tokens, base URLs, file paths — is
implementation detail behind those three calls.

| Implementation | `list()` | `fetch()` | `artifact()` |
| --- | --- | --- | --- |
| Embedded | the binary's asset table | bytes compiled into the binary | n/a: nothing embedded references |
| Local file | the one document supplied | that document | `artifacts/<sha256>` beside it |
| Hub local | the blessed shelf, read in-process | the stored artifact | `<hub root>/artifacts/<sha256>` |
| Hub public | a placed hub service's store over HTTP — any instance's, including this one's own | that service's artifact endpoint | `remote/assets/{id}/{version}/artifacts/{sha256}` |
| Static repository | `zebflow-repository.json` | a document path from the index, pinned by digest | `<base>/artifacts/<sha256>` |

This matters because it means a new source is a **fetcher**, not a new package
format, a new installer, or a new review path. Adding static repositories should
add one implementation and nothing else. If it requires touching the installer,
the abstraction is in the wrong place.

The same reasoning applies to RWE libraries: a library resolved from any
serving produces the same lock entry shape, differing only in `source`. The
binary is only the seed (§2), never a serving a lock can name; the shelf it
seeds is the implementation that is always available offline.

**That is now what the code does.** `ProjectHubRepository` and
`PlatformHubRepository` were each hardwired to a remote Zebflow instance —
`base_url`, `remote_owner`, `remote_project`, `read_token`, none of which a
static repository has. Both now convert into one `HubRepositoryRef`, and
`HubRepositoryChannel` in `src/platform/services/hub_repository.rs` is the
interface above: `list`, `fetch`, and `artifact_url`, with one variant per
channel. `HubService` calls those three and knows nothing else about where a
package came from.

The claim that adding a source should not touch the installer is checkable, and
it holds: `fetch_platform_project_bundle` returns a `HubPackageSpec` and a
channel, and everything after it — `ProjectBundleInstallPlan::build`, the safety
review, `refuse_unreviewable_project_bundle`, `PreparedProjectBundle`,
`install_project_bundle` — is the same code it was, unchanged, and cannot
observe which channel produced the document.

### Static repositories

A static repository is any HTTPS location that serves an index and a set of
package documents. It is deliberately **not** "a GitHub repository": GitHub is
the most convenient host, not a special case, and the same fetcher works against
GitLab, an object store, a plain web server, or a local path.

```text
https://<host>/<path>/
  zebflow-repository.json          index: packages, kinds, versions, document paths
  packages/
    acme-tools/1.0.0/package.json  a HubPackage document
    data-table/2.1.0/package.json
  artifacts/
    <sha256>                       bytes a package references rather than carries
```

One fetch of the index for discovery, then direct fetches of the documents it
names. No vendor API, no rate limits, and no token for a public repository.

This carries nothing new: a package document is the same `HubPackage` envelope a
local file install already accepts. A repository is a transport, not a format.

The `artifacts/` directory is the same content-addressed layout every other
channel keeps — `<channel-base>/artifacts/<sha256>` — so a package that
references bytes instead of carrying them is installable from a static
repository too. The interface table above listed that call as not implemented
for this channel; it was the one place a static repository would have been a
lesser channel, and implementing it cost nothing, because the reference resolver
already takes the location from whichever channel served the document.

#### The index

`zebflow-repository.json` is a registered contract kind, `HubRepositoryIndex`,
defined at [`kinds/hub-repository-index/README.md`](./kinds/hub-repository-index/README.md).
It is a durable format other people publish, so it is versioned, strictly
decoded, and refuses what it cannot mean:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "HubRepositoryIndex",
  "metadata": { "name": "zebflow-hub" },
  "spec": {
    "packages": [
      {
        "package_id": "zebflow.kids-educational-games",
        "asset_kind": "project_bundle",
        "title": "Kids Educational Games",
        "description": "Number and letter games for young children.",
        "latest_version": "1.0.0",
        "releases": [
          {
            "version": "1.0.0",
            "path": "packages/zebflow.kids-educational-games/1.0.0/package.json",
            "sha256": "01dcc1463d3113bf2d7ac2ccedd28742d4cdef24755c118706b8689af48994ba",
            "size_bytes": 265
          }
        ]
      }
    ]
  }
}
```

It carries **no package content**. Discovery costs one file; a release costs one
more, and nothing serving either has to run code.

**Mutability is the risk that has to be answered.** A git tag can be moved and a
static file can be replaced, so a version string alone is not an identity.

The index answers it for the fetch. `releases[].sha256` is **required**, and the
fetch refuses a document whose bytes hash to anything else, so a file replaced
under its path fails closed rather than installing whatever is there now. The
index and the document must also agree on who they are — the document's
`metadata.name` and `metadata.version` are checked against the entry that
pointed at it — so an index cannot aim one name at another package's bytes and
pass the digest check by pinning them.

Two honest limits sit beside that, and neither is closed by this section.

The index is itself mutable. Republishing it with a new digest for new bytes at
the same version is indistinguishable, to a reader, from a legitimate release.
What the digest buys is that the *path* stops being the identity: nothing
between the index and the document can be swapped without detection.

And the earlier draft of this paragraph said the answer was to "record the digest
in `zeb.lock`". That is true of the resources §4 covers — RWE libraries and node
bundles — and **not** of a platform-scope project bundle install, which records
no digest anywhere: `ProjectBundleInstallResult` carries destinations and
pipelines and no provenance at all, and the `zeb.lock` a bundle installs is the
publisher's lock for *its* dependencies, not a record of the bundle itself.
Whichever channel it came from, an installed project cannot say what it was
installed from. That is a gap in §4 rather than in this channel, and it is
recorded in §7.

#### Resolution order

`zeb install <ref>` with no `--repo` searches the configured sources **in order,
and the first hit wins.** A fresh instance seeds exactly two, and this is the
order:

| # | Source | Kind | Where |
| --- | --- | --- | --- |
| 1 | `zebflow-hub` | static repository | `https://raw.githubusercontent.com/zebflow/hub/main` |
| 2 | `zebflow-com` | API hub | `https://hub.zebflow.com/api` |

The static repository is first because it needs no server and static file
hosting absorbs install traffic that would otherwise hammer the API hub. The
API hub is second, as the source that can answer about publisher identity,
grants, and retraction (decided 2026-08-27; the seeded priorities were
previously reversed).

**The order is configured, not compiled in.** It is
`PlatformHubRepository.priority`, ascending, with `repository_id` breaking ties;
the two official sources are seeded at 10 and 20 and anything added later
defaults to 100, behind both. The listing endpoint returns the sources in that
order alongside the packages, so the client resolves by walking the list the
instance handed back rather than ranking anything itself.

Ordering settles only the question ordering can settle:

- Two **sources** offering one package id → the first one searched wins, and
  `--repo <id>` names the other.
- Two **publishers** within one source offering one slug — `acme.quiz` and
  `globex.quiz` for the reference `quiz` — → refused, naming both full ids.
  They are equally near, so nothing about order answers it.

The second rule is the one this document already had. The first replaces a rule
that refused both cases, which was wrong once there were two official sources:
a package present in both — the normal state of any migration between them —
would have failed rather than resolved.

**A reference no source carries is one failure, so it is one error**, naming
every source that was searched, in order, and saying which of them did not
answer. A source that is unreachable is reported as unreachable rather than
being dropped, because "nobody publishes it" and "the only source that does is
down" are different answers and used to be told to the user as one.

**Trust.** A static repository is not meaningfully more dangerous than a remote
Hub: both execute someone else's bytes and both run the same review. What a Hub
adds is publisher identity and grants, which is provenance rather than
behaviour. Three violation detectors now refuse rather than report — a file type
no project accepts, a bundle declaring a node kind this build already provides,
and a pipeline whose bytes the review cannot read — and all three read the
package rather than judging what it does. No detector refuses a pipeline for its
behaviour, so a static repository still buys exactly what any other channel does:
one review, and no protection from a package that is honest about being
hostile. The condition this paragraph originally set — design static
repositories now, enable them once the first detectors land — is met, and they
are enabled.

Enabling them added no gate and skipped none. A static repository's document
reaches `ProjectBundleInstallPlan::build` exactly as a remote hub's does, so
`refuse_unreviewable_project_bundle` runs over its entries and
`refuse_prepared_install_violations` over its prepared bytes, and a package the
review refuses has no prepared work at all whichever channel carried it. The
channel-specific code stops at `fetch`.

Three things the channel does add, because a URL a user named is a weaker claim
than an instance's own hub:

- **Redirects are refused, not followed.** The URL is the whole trust decision,
  and a redirect moves it to a host the egress check never saw.
- **Every fetch is bounded before it starts** — the index by the contract's own
  ceiling, a document by the size the index declares — so a repository cannot
  make a client download something it did not ask for.
- **A release path may only descend.** `..`, an absolute path, and a backslash
  are each refused by the index validator, because a path from the index is
  joined onto the base URL the user trusted.

## 3. One review, every channel

Every channel that installs into a project runs the same package review from
`src/platform/policy/package.rs`, producing the same `PackageSafetyReview`.

That holds down to how one entry is read. A channel differs in whether it can
obtain an entry's bytes at all — carried inline, fetched from a hub, resolved
beside a document, already staged for writing — and in nothing after that:
`PackagePolicyEntry::from_bytes` is the only thing that turns bytes into text the
review can scan or into a recorded reason it cannot, so two gates cannot reach
different verdicts on one document by each reading it their own way.

A Hub package is not safer than a local file. It is published, which is a
statement about provenance, not about behaviour. Treating them differently would
mean the safest-looking path had the weakest checks.

The review has three gates, described in full in
[`kinds/node-bundle/README.md`](./kinds/node-bundle/README.md#security):

1. the contract validator refuses malformed documents
2. policy **violations** refuse unsafe ones, and are never overridable
3. policy **warnings** are reported for the user to accept

Every install surface can be asked for that verdict before it acts, and the
verdict a review shows is the verdict the install enforces. A project bundle
reaches that literally: review and install share one `ProjectBundleInstallPlan`,
so asking what the install would do and asking whether it may run are one
question. Elsewhere they are two readings that cannot disagree, because both run
over the same bytes at the same destinations through the same reader.

## 4. Identity and integrity

Every distributed resource needs three things, and the kind that owns its format
must supply all three:

| Property | Question | Where it lives |
| --- | --- | --- |
| Identity | what is this, exactly? | package slug plus release version |
| Integrity | are these the bytes that were published? | SHA-256 digest |
| Provenance | where did it come from? | `DependencyLock` source and `source_id` |

`DependencyLock` is frozen and already records all three for RWE libraries and
node bundles. A new distributable resource should extend that lock rather than
invent a parallel record.

## 5. Reproducible versus carried

This is the distinction that decides what an export must contain.

A **reproducible** resource can be fetched again from its recorded source. The
lock is enough; the bytes need not travel.

A **carried** resource cannot. A bundle from a private repository or a local
file is nameable and verifiable but not obtainable, so an export that omits it
produces a project that cannot be made whole.

| Source | Reproducible | Export must carry bytes |
| --- | --- | --- |
| `hub.local` | yes on this instance — the shelf never deletes; across instances, only for coordinates the receiving release seeds | no (a transfer to an unknown release should carry bytes) |
| `hub.public` | if the remote hub is reachable and still grants | no |
| `hub.static` | if the URL is still alive | no |
| `direct.npm` | weakly — the npm coordinate in `source_id` can be re-converted | **yes** |
| `direct.file` / `project` | **no** | **yes** |

Where bytes cannot travel and are not carried, the receiver must be told
precisely what is missing. For node bundles this is what
`repo/nodes/{kind}.json` provides: the interface survives so the graph stays
readable and the node can be reimplemented. Other resources need an equivalent
answer or an explicit statement that they have none.

## 6. Where distributed bytes land

The directory tree and tiers are owned by
[`instance-directory.md`](./instance-directory.md) — one source, not repeated
here. Distribution's only rule: an installed dependency lands in the INSTALLED
tier (`data/hub/`); authored content a project adopts (`pipeline_bundle`,
`template_bundle`, `add`) lands in `repo/`.

## 7. Open decisions

These belong to distribution rather than to any single kind, and are recorded
here so each kind's review does not settle them separately.

**Git source installation.** Installing a library or bundle directly from a
public repository was deliberately deferred: the safety surface is large and the
trust basis is unclear. `spec.hosts` and the `violations` tier are the shape of
an answer but are declared, not enforced. Decide during the `RweLibraryManifest`
review, and apply the decision to every channel at once.

**Promotion into the curated namespace.** A third-party package adopted by
Zebflow moves from `n.x.acme.thing` to `n.acme.thing`, which is a rename and
therefore a breaking change. Promotion must be a deliberate versioned event, or
must not happen to packages authored by others.

**Reproducibility of official content.** Resolved for RWE libraries and the UI
template sets: the seeder — every boot, check-first (§1b) — publishes the binary's blessed content
into the local hub as `zebflow.*`, so an installed library carries a real
`zeb.lock` entry (`source: hub.local`, digest of the installed bytes) and a template
set is an ordinary reviewed package. What remains open is official *node
bundles*, which are still embedded with no lock entry. A lock a project wrote
before the seed existed, still saying `embedded`, is simply invalid and
regenerates through ordinary resolution
([`kinds/dependency-lock/README.md`](./kinds/dependency-lock/README.md)).

**A platform-scope install records no provenance.** Nothing written by a project
bundle install says which package, version, digest, or repository produced it.
§4 requires identity, integrity, and provenance of every distributed resource
and `DependencyLock` supplies all three — for libraries and node bundles. A whole
project has no equivalent record, so an installed app cannot be re-obtained,
verified, or updated, and the digest a static repository pins protects only the
fetch that used it. Found while implementing static repositories; it belongs to
every channel equally and is not fixed here.

**Intra-project references.** `n.function.call` targets another pipeline by
slug and nothing verifies the target resolves, so an export that omits it fails
only at run time. Recorded against
[`ProjectBundle`](./kinds/project-bundle/README.md).

## 8. Freeze checklist for a distributable resource

Before a kind that participates in distribution is frozen:

- [ ] identity, integrity, and provenance are all recorded
- [ ] every channel that can install it runs the one shared review
- [ ] it is classified reproducible or carried, and export behaves accordingly
- [ ] where it cannot be carried, the receiver is told exactly what is missing
- [ ] its bytes land in the area matching what it is, declared or materialised
- [ ] uninstall removes only what that resource owns
- [ ] a failed install leaves the previous state usable
