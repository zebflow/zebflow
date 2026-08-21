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
zebflow run <project-or-hub-asset-url>  materialise the project, then serve it
```

Each of the two HTTP surfaces has a review sibling that reports what the install
would do and does none of it:

```text
POST /api/users/{owner}/hub/install/review
POST /api/platform/hub/install/review
```

They take the same body as the install, including the three consent flags, and
answer with the destinations that would be written, the pipelines that would be
registered and activated, what the install-time SQL does, the safety review, and
whether the package is installable at all. `zebflow run` has no review step.

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

The CLI reflects the same split: `zebflow run` is platform scope, taking a
project or Hub asset URL and materialising it. There is no project-scope CLI
verb today; project-scope distribution is API and UI only.

## 0b. Consumption modes

Scope says *where* something arrives. Mode says *why*, and the two are
independent.

| Mode | Command | What the user sees |
| --- | --- | --- |
| **Run** | `zeb app run <ref>` | the app, served at its entry point |
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

Run mode is used by people who never chose a data directory, so it needs a
default and must not ask. `PlatformConfig` already supplies one:
`.zebflow-platform-data`, overridable with `ZEBFLOW_PLATFORM_DATA_DIR`.

That default is **relative to the working directory**, which is right for a
developer running a server in a project folder and wrong for someone who typed
`zeb app run` from anywhere. Run mode needs a stable per-user location, resolved
once, so that running the same app from two different directories does not
install it twice.

```text
zeb app run osgeo-spatial-tool        obtain if needed, then serve it
```

Someone who wants a geospatial selection tool obtains one thing and uses it.
That Zebflow is underneath is not their concern, and this is the mode that makes
distribution worth having: obtaining a working tool should cost one command.

`zebflow run` implements this today, and its usage line — "materialize one app
project if needed, then serve its public route" — is exactly this mode.

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

**This section is a proposal, not a description.** The only distribution verb
that exists today is `zebflow run`, which materialises a project or Hub asset
and then serves it. Everything else below is unimplemented and is written here
so the surface is designed once rather than grown one flag at a time.

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

That is not a new choice. `zebflow project config migrate` and `zebflow k8s
cluster init` already have this shape, and `zebflow run` is already a blessed
top-level verb. The design below continues an existing pattern rather than
introducing one.

The deciding argument is scope. `project install` and `node install` mean
different things — one creates a project, the other changes one — and putting
the noun first is what makes that visible at the point of typing. Verb-first
would put the ambiguous word first and the disambiguating word second.

### The surface

```text
# Platform scope — creates a project
zeb project install <ref> [--repo <url>]   materialise a project into this instance
zeb project export <kind>                  write a transfer archive
zeb run <ref>                              materialise if needed, then serve

# Project scope — changes the current project
zeb node install <ref> [--repo <url>]      take on a node bundle dependency
zeb node uninstall <kind>
zeb lib add <ref> [--repo <url>]           resolve an RWE library
zeb add <ref> --to <folder>                copy content in as this project's source
zeb publish <source> --to <hub>            share outward
```

`project install` creates something; `node install` changes something that
already exists. Reading the noun tells you which, which is the property the
scope split in §0 exists to protect.

**Context.** Project-scope commands need to know which project. `--owner` and
`--project` already exist as flags, and `default_owner` and `default_project`
already exist in the CLI configuration, so the resolution order is settled:
explicit flag, then configured default, then error. No command guesses.

**A reference must name its channel.** `<ref>` alone is ambiguous across the
channels in §2, and the channel is the trust decision, so it is never inferred:

```text
zeb node install acme-tools                     this instance's Hub
zeb node install acme-tools --repo <url>        a static repository
zeb node install ./acme-tools.json              a local file
```

### Blessed top-level verbs

One verb earns a top-level place, because it is the headline: obtaining a whole
project should be one short command.

```text
zeb install <ref>        alias for: zeb project install <ref>
```

It is an **alias**, not an inference. Bare `install` always means the same thing
regardless of any configured project context, because a command whose meaning
changes with hidden state is the failure §0 exists to prevent.

### Binary name

The long name is `zebflow`; the short name is `zeb`. Shortening is the norm —
Kubernetes ships `kubectl`, Google Cloud ships `gcloud`, Cloudflare ships
`wrangler` — and `zeb install` is the shape people will actually type.

Both names should exist, with `zeb` as the primary and `zebflow` kept working,
so that installing a product called Zebflow and finding only a `zeb` binary is
never surprising. `zeb` also matches the `zeb/*` library namespace and the
`zeb.lock` file, so the short name is already the project's own vocabulary.

## 1. What is distributable

Unless a row says otherwise, these are project-scope acts.

| Resource | Kind that owns its format | Channels | Direction |
| --- | --- | --- | --- |
| Node bundle | `NodeBundle` | Hub asset, remote pack, local file, project transfer | export, publish, install |
| RWE library | `LibraryManifest` | embedded in binary; Hub and Git planned | install only, today |
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

These are project-scope verbs. Platform-scope install is described in §0 and
behaves differently: it produces a whole project, which the receiving instance
then owns and edits freely. At project scope `install` means *managed*; at
platform scope it means *materialised and yours*.

| Verb | What it means | Lands in | Editable by the receiver | Recorded in `zeb.lock` | Removal |
| --- | --- | --- | --- | --- | --- |
| **Install** | takes on a managed dependency | `data/` | no | yes | uninstall |
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
| Hub install, node bundle | not chosen | `data/nodes/{package}` |
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

## 1b. Publish targets

"Publish" is two different acts and must not be written as one:

| Target | Route | Who can see it | Trust basis |
| --- | --- | --- | --- |
| **Local Hub** | `hub/assets/publish` | this instance, subject to its access rules | the publishing instance itself |
| **Remote Hub** | `hub/remote/assets/publish` | another instance over HTTP | publisher token plus a repository grant |

Publishing to a local Hub shares within one deployment. Publishing to a remote
Hub sends bytes to a system you do not control, which is an outward-facing act
with a different consent requirement.

## 2. Channels

A channel is a way bytes move. Each has a different trust story, and that is the
reason they are listed separately rather than treated as one "install".

| Channel | Source of bytes | Trust basis | Today |
| --- | --- | --- | --- |
| Embedded | the Zebflow binary | the release itself | libraries, official node bundles |
| Local file | a document the user supplies | the reviewing user | node bundles |
| Hub asset | this instance's Hub store | publisher identity + stored digest | all asset kinds |
| Remote pack | another instance's Hub over HTTP | repository grant + artifact digest | packs, projects |
| Transfer archive | an export file | whoever produced it | project bundle, files |
| Git remote | a git repository | the remote's own access control | project `repo/` |
| Static repository | an HTTPS location serving an index and package documents | the repository URL the user named, plus a locked digest | **not implemented** |

**Embedded is not a channel a user invokes.** It is listed because it is how
official content arrives, and because a resource moving from embedded to
installed is a distribution change even though no bytes travel.

### One repository interface

The channels above are not five mechanisms. They are five **implementations of
one interface**, and the format they carry is the same in every case.

A repository answers two questions:

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
| Zebflow Hub | this instance's asset store | the stored artifact | `<hub root>/artifacts/<sha256>` |
| Remote Hub | another instance's HTTP API | that instance's artifact endpoint | `remote/assets/{id}/{version}/artifacts/{sha256}` |
| Static repository | `zebflow-repository.json` | a document path from the index | not implemented |

This matters because it means a new source is a **fetcher**, not a new package
format, a new installer, or a new review path. Adding static repositories should
add one implementation and nothing else. If it requires touching the installer,
the abstraction is in the wrong place.

The same reasoning applies to RWE libraries: embedded is not a special case, it
is the implementation that happens to always be available offline. A library
resolved from the binary and one resolved from a repository produce the same
lock entry shape, differing only in `source`.

**What exists today does not yet have this shape.** `ProjectHubRepository` is
hardwired to a remote Zebflow instance — it carries `base_url`, `remote_owner`,
`remote_project`, and `read_token`, none of which a static repository or a local
file has. Generalising it is the work that makes the rest of this section
implementable, and it should happen before a second remote source is added
rather than after.

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
```

One fetch of the index for discovery, then direct fetches of the documents it
names. No vendor API, no rate limits, and no token for a public repository.

This carries nothing new: a package document is the same `HubPackage` envelope a
local file install already accepts. A repository is a transport, not a format.

**Mutability is the risk that has to be answered.** A git tag can be moved and a
static file can be replaced, so a version string alone is not an identity. The
answer is the one already used everywhere else: resolve, hash, and record the
digest in `zeb.lock`. A document that changes under a version then appears as a
digest mismatch and fails closed, exactly like a tampered bundle.

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
repositories now, enable them once the first detectors land — is met; whether to
enable them is a separate decision and still open.

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
| `embedded` | yes, by the matching Zebflow release | no |
| `hub` | yes, if the Hub is reachable | no |
| `project` / local file | **no** | **yes** |

Where bytes cannot travel and are not carried, the receiver must be told
precisely what is missing. For node bundles this is what
`repo/nodes/{kind}.json` provides: the interface survives so the graph stays
readable and the node can be reimplemented. Other resources need an equivalent
answer or an explicit statement that they have none.

## 6. Where distributed bytes land

Distribution follows the project's ownership model:

| Area | Meaning | Example |
| --- | --- | --- |
| `repo/` | what the human declares | `zeb.lock`, pipelines, node interfaces |
| `data/` | what the machine materialises | installed node bundles |
| `files/` | what the application stores for users | ZebFS objects |

An installed dependency is materialised from a declaration, so its bytes belong
in `data/` while the declaration stays in `repo/`. A resource that is *authored*
by the receiving project — a pipeline, a template, a folder of source — is the
opposite: it lands in `repo/` and becomes that project's own work.

That difference is why `node_bundle` installs under `data/` while
`pipeline_bundle` and `template_bundle` install under `repo/`.

## 7. Open decisions

These belong to distribution rather than to any single kind, and are recorded
here so each kind's review does not settle them separately.

**Git source installation.** Installing a library or bundle directly from a
public repository was deliberately deferred: the safety surface is large and the
trust basis is unclear. `spec.hosts` and the `violations` tier are the shape of
an answer but are declared, not enforced. Decide during the `LibraryManifest`
review, and apply the decision to every channel at once.

**Promotion into the curated namespace.** A third-party package adopted by
Zebflow moves from `n.x.acme.thing` to `n.acme.thing`, which is a rename and
therefore a breaking change. Promotion must be a deliberate versioned event, or
must not happen to packages authored by others.

**Reproducibility of official content.** Official node bundles and libraries are
embedded in the binary and get no lock entry, so a project using them records
nothing about what it depends on. Moving that project to an instance on a
different Zebflow release changes its dependencies silently. Either official
content becomes locked like anything else, or the lock records the runtime
version it assumes.

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
