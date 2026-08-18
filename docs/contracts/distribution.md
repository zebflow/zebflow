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

## 0a. Command line surface

Zebflow's premise is that obtaining something should be one command, the way a
package manager works. Someone who wants a video pipeline built from ElevenLabs
and Seedance nodes, or a set of office tools, should be able to obtain the whole
thing and have it belong to their instance.

**This section is a proposal, not a description.** The only distribution verb
that exists today is `zebflow run`, which materialises a project or Hub asset
and then serves it. Everything else below is unimplemented and is written here
so the surface is designed once rather than grown one flag at a time.

The command names its noun, and the noun determines the scope. This follows the
group-and-verb shape used by cloud CLIs, and matches the existing `zebflow
project ...` and `zebflow k8s cluster ...` groups.

```text
# Platform scope — creates a project
zebflow project install <ref>          materialise a project into this instance
zebflow project export <kind>          write a transfer archive
zebflow run <ref>                      materialise if needed, then serve

# Project scope — changes the current project
zebflow node install <ref>             take on a node bundle dependency
zebflow node uninstall <kind>
zebflow lib add <ref>                  resolve an RWE library
zebflow add <ref> --to <folder>        copy content in as this project's source
zebflow publish <source> --to <hub>    share outward
```

`project install` creates something; `node install` changes something that
already exists. Reading the noun tells you which, which is the property the
scope split in §0 exists to protect.

**Context.** Project-scope commands need to know which project. `--owner` and
`--project` already exist as flags, and `default_owner` and `default_project`
already exist in the CLI configuration, so the resolution order is settled:
explicit flag, then configured default, then error. No command guesses.

**A reference is not always a URL.** `<ref>` has to name the channels from §2 —
a Hub asset, a remote repository asset, a local file, a git source — and the
reference syntax must make the channel explicit rather than inferred, because
the channel is the trust decision.

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
| Hub add | `target_folder`, chosen by the receiver | `pipelines/hub/{id}` for pipelines and RWE source, `hub/{id}` otherwise |
| Hub install, node bundle | not chosen | `data/nodes/{package}` |
| UI catalog add | **not offered** | fixed by the catalog |

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
| Git source install | a public repository | **undecided** | not implemented |

**Embedded is not a channel a user invokes.** It is listed because it is how
official content arrives, and because a resource moving from embedded to
installed is a distribution change even though no bytes travel.

## 3. One review, every channel

Every channel that installs into a project runs the same package review from
`src/platform/policy/package.rs`, producing the same `PackageSafetyReview`.

A Hub package is not safer than a local file. It is published, which is a
statement about provenance, not about behaviour. Treating them differently would
mean the safest-looking path had the weakest checks.

The review has three gates, described in full in
[`kinds/node-bundle/README.md`](./kinds/node-bundle/README.md#security):

1. the contract validator refuses malformed documents
2. policy **violations** refuse unsafe ones, and are never overridable
3. policy **warnings** are reported for the user to accept

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
