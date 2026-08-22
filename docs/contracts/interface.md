# Interface

Status: **draft**. The vocabulary is proposed. §8 is the work that remains.

This is a contract, not a kind. It defines no document format. It defines the
**words** Zebflow uses for the things a person can do, so that a terminal, a
web UI, and a future desktop launcher are three renderings of one vocabulary
rather than three products that drifted apart.

A term added by whoever ships a button is how two glossaries begin. This
document is where a term is added.

## 1. Why the words are the contract

Nouns can be added forever. A verb someone has typed into a script, a README, or
a blog post is permanent — it outlives the code that implemented it and the
person who chose it.

So the verbs are frozen and the nouns grow. A new capability becomes
`zeb <newnoun> <verb>` and never a new top-level word. That rule is what kept
`gcloud` coherent for a decade and what `npm` lost when `ci`, `exec`, and
`dedupe` each arrived at the top level.

## 2. Shape

**Noun group first, verb second, with a small number of blessed top-level
verbs.** `distribution.md` chose this before this document existed, comparing
`kubectl`, `docker`, `aws`, `gcloud`, and `wrangler`, and settling on
**`gcloud` + `wrangler`**. This document does not re-open that.

An earlier draft here argued for the `gh` shape instead — strictly two levels.
It was wrong on the facts. The everyday commands are identical under both, and
they differ only on the three-level ones: `zeb k8s cluster set-replicas` and
`zeb project config migrate`. Flattening those loses information a general user
never sees and an operator relies on — `zeb cluster set-replicas` does not say
what kind of cluster. Recorded so the argument is not had a third time.

Rejected, with reasons, so they are not re-proposed:

- **`kubectl`'s verb-first model** is the cleanest design in this space, and it
  does not fit. It works when every noun supports every verb, so a new resource
  type gains the whole verb set on the day it is defined. Zebflow's operations
  are not uniform CRUD: `publish`, `activate`, `migrate`, `review`, and `retract`
  do not collapse into get/create/delete, and forcing them would make the verbs
  describe something other than what they do.
- **`docker`'s dual surface** — `docker ps` beside `docker container ls` — is a
  wart it grew, not a model. §4 permits aliases, but only with an exact
  canonical expansion, which is the property docker's shortcuts lack.
- **`git` / `npm` / `cargo`'s flat verbs** work because the noun is implicit: the
  current repository, the current package. Zebflow has no implicit noun. One
  instance holds many projects and a directory is not one of them (§5).

## 3. The four groups

Every command belongs to exactly one.

### Group 1 — Server modes

Bare verbs, no nouns, configured by environment. These are "be a server", not
"do something to something", which is why they are the one deliberate exception
to `noun verb`. `docker`, `caddy`, and `nginx` all behave this way and nobody
is surprised.

```
zeb                    standalone: controller + office
zeb controller         control plane
zeb office             execution plane
```

### Group 2 — General use

What a person who has never read this document types. Blessed top-level verbs,
each an alias with an exact canonical expansion (§4).

```
zeb install <ref>      materialise a project  (= zeb project install)
zeb run <ref>          materialise if needed, then serve
zeb add <ref> --to <folder>     copy content in as this project's source
zeb publish <source> --to <hub>
zeb list · zeb status
```

The set is larger than it should be. `install` and `run` clearly earn a
top-level place; `add` and `publish` do not say *what* is being added or
published without their noun, and reading them left to right tells you less than
`zeb hub publish` would. That is a disagreement to settle with
`distribution.md`, which blesses them, rather than a decision this document
makes alone.

### Group 3 — Project maintenance

Already exists, already correct in shape, and already offline: these run
directly against a data directory with no server, because they are what you run
*when the server will not start*. One of them is why a server refused to boot on
2026-08-20 until its configuration was explicitly migrated, which was correct
behaviour.

```
zeb project config    migrate <owner> <project>
zeb project lock      migrate <owner> <project>
zeb project pipelines migrate <owner> <project>
```

This group is three levels deep, and stays that way: `config`, `lock`, and
`pipelines` name genuinely different documents.

### Group 4 — Kubernetes provisioning

Ten commands, a third of the current CLI, that a general user never meets. It
stays a deep noun group and is never promoted.

```
zeb k8s cluster init|describe|validate|set-image|set-replicas|…
```

## 4. Aliases

An alias is permitted only when it names one canonical command exactly. The
alias is sugar; the canonical form is the vocabulary.

| Alias | Expands to |
| --- | --- |
| `zeb install <ref>` | `zeb project install <ref>` |
| `zeb remove <ref>` | `zeb project remove <ref>` |
| `zeb list` | `zeb project list` |

The alias set is frozen at the four verbs in Group 2. Adding a fifth is a
change to this document.

## 5. There is no current directory

A Zebflow project lives at `<data-root>/users/{owner}/{project}/`. It is owned
by the instance, not by a folder you `cd` into. Nothing about the shell's
working directory identifies a project.

So context is explicit and stated, the way `kubectl` and `gcloud` learned to do
it, rather than inferred from where the user happens to be standing.

```
zeb login http://localhost:10610
zeb use superadmin/default
zeb status
```

**Resolution is explicit, and nothing guesses:**

```text
--owner / --project flag  →  stored client default  →  error
```

**The stored client default does not exist yet.** `distribution.md` says
`default_owner` and `default_project` "already exist in the CLI configuration".
They exist, but as *server* configuration read from
`ZEBFLOW_PLATFORM_DEFAULT_OWNER` and `ZEBFLOW_PLATFORM_DEFAULT_PROJECT`, which
name the owner and project the server bootstraps on first boot. They are not a
client-side record of which project a person is working on, and no `zeb config
set` writes one.

So a client context store — instance, credential, owner, project — is a
prerequisite for the everyday path being short, and it is unbuilt. Until it
exists, project-scope commands require their flags.


Project context is **not** defaulted, because bare `zeb install` does not consult
it (§7). `zeb install kids-educational-games` needs no setup for that reason, not
because an unset context is filled in — the command materialises a project named
by the reference, and the review prints before anything is written.

Context matters for the scoped forms, `zeb project install` inside an existing
project among them, where it is stated rather than inferred.

## 6. Transport

**HTTP when the server owns the state; direct when the command exists to repair
a server that will not start.**

Groups 2 and 4 speak to a running instance over its API — the same routes the
web UI uses, which is what keeps the two surfaces honest. Group 3 runs directly
against the data directory.

The reason is not taste. An install touches ten things beyond writing files,
including `node_registry.refresh_project`, which is an in-memory cache inside the
running server. A CLI that wrote to disk behind a live server would leave it
believing something false about its own nodes.

The UI is a rendering of this vocabulary over the same HTTP API. It does not
shell out to the CLI: that would make process spawning, argument escaping, and
output parsing into a permanent dependency between two surfaces that only need
to agree on words.

## 7. What `zeb install` expands to

Settled, in `distribution.md` §on verbs, before this document existed:

```text
zeb install <ref>        alias for: zeb project install <ref>
```

`install` means materialise a project at platform scope, and take on a managed
dependency at project scope. **Bare `install` always means the first**, whatever
project context is configured.

It is an alias, not an inference. That is deliberate: a command whose meaning
changes with hidden state is the failure the distribution contract opens by
naming. An earlier draft of this document proposed exactly that inference --
create a project when no context is set, add a dependency when one is -- which
was wrong, and is recorded here so it is not proposed a third time.

Project creation is irreversible and routes through the platform-scope install
review, which reports the SQL a package would run, the pipelines it would
register, and what it would activate.

## 8. What is not yet mapped

This document proposes a vocabulary. It has not yet been checked against the
222 API routes, 37 MCP tools, and 27 CLI commands that exist, so "covers all
current operations" is a goal here and not yet a fact.

The noun groups below were read off the route table rather than invented, and
are the starting point for that mapping:

```
hub 34 · pipelines 28 · templates 24 · db 21 · files 13 · transfer 10
nodes 10 · git 10 · settings 8 · credentials 8 · docs 7 · rwe 5
mapserver 5 · tables 4 · assets 4 · mcp 3 · assistant 3
```

Eighteen groups is more than a person can hold. Several collapse — `tables` into
`db`, `assets` and `rwe` into source, `transfer` and `git` into `project` — and
the reduction is the work. The deliverable of that pass is the list of
operations **no proposed term covers**, because without it minimality and
completeness are both claims nobody can check.

`zebflow run <url>` is the first thing that mapping must resolve. It materializes
a project from a hub asset and then serves it, which is half Group 2 and half
Group 1, and it is the existing name most likely to collide with §7.
