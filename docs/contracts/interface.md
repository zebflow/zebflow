# Interface

Status: **draft**. The vocabulary is proposed. §8 has now been checked against
the code and reports what it does not cover.

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

Bare verbs, configured by environment. These are "be a server", not "do
something to something", which is why they are the one deliberate exception to
`noun verb`. `docker`, `caddy`, and `nginx` all behave this way and nobody is
surprised. Only `run` takes an argument, and it takes it for the same reason
`nginx -c` does: to say which thing this process is serving.

```
zeb                            standalone: controller + office
zeb controller                 control plane
zeb office                     execution plane
zeb run <owner>/<project>      serve one installed project's public route
```

`run` is the one server mode that names what it serves, and it is a server mode
rather than a Group 2 verb because that is all it does: it starts a process. It
used to also materialise a project from a hub asset URL, which made it half of
each group and put a second materialisation path beside `install` — §8 named
that collision and this is its resolution. `run` now refuses a project that is
not installed and names `install` as the command that installs it, so there is
exactly one way a project comes into existence and it is the reviewed one.

**`run` starts a server, and it is the command whose stated job that is.** That
matters in the other direction too, since §6 lets a Group 2 command open the
data root itself when nothing is listening. Such a command does its work in its
own process and exits; it never leaves a server behind, because a daemon started
as a side effect of an install is a process the person did not ask for and
cannot see. When they want one, they say so, and the words for saying so are the
four in this group.

**`master` and `worker` are deprecated spellings** of `controller` and `office`.
The binary has always accepted both and documented neither, which is the docker
wart in §2: two words for one role, with nothing telling a reader which is the
real one. They are not dropped outright because a word already typed into a
script outlives the code that chose it, and they are not left silent either —
each names exactly one canonical word (§4's test), prints a deprecation line
naming it, and is listed in `zeb help` as deprecated. They are removed when this
document leaves draft. No third spelling is ever added.

**A server mode that cannot perform its role refuses to start.** A process whose
whole purpose is to join a control plane must not serve traffic outside one, so
`zeb office` and `zeb controller` validate their cluster configuration before
opening the data root, and the error names *every* missing variable at once
rather than one per restart. §3a says which variables those are.

### Group 2 — General use

What a person who has never read this document types. Blessed top-level verbs,
each an alias with an exact canonical expansion (§4).

```
zeb install <ref>                    materialise a project  (= zeb project install)
zeb remove <owner>/<project>         delete a project       (= zeb project remove)
zeb list · zeb status
zeb login <instance-url> · zeb use <owner>/<project> · zeb logout
```

`login`, `use`, and `logout` are the client context store (§5), which has no
noun to sit under: it configures the client itself rather than acting on
anything the instance holds, so `zeb context set` would name a resource that
does not exist.
`gcloud` and `kubectl` both landed on bare `auth`/`config` verbs here for the
same reason. `logout` is the only one of the three §5 did not already name, and
it is the counterpart that revokes what `login` stored; removing a credential
must not require deleting a file by hand.

`remove` is the counterpart to `install`, and it is a blunter word than it
looks: it deletes the project, its source, its database, and the files it
stored. It is **not** an uninstall. `distribution.md` §0 says a platform-scope
install "produces a project and cannot be undone by an uninstall" — what
arrived belongs to the receiving instance and nothing records which bytes came
from the package — so there is nothing to undo and the only removal that exists
is deleting the whole project. The command says so before it asks.

**`add` and `publish` are no longer blessed here, and are not built.** Neither
says *what* is being added or published without its noun, and reading them left
to right tells you less than `zeb hub publish` does. That was recorded as a
disagreement with `distribution.md`, which blessed them; the disagreement is
settled in favour of the nouns, and both documents now spell them
`zeb hub add <ref> --to <folder>` and `zeb hub publish <source> --to <hub>`,
under the `hub` group that already owns the routes they would call. Neither
exists in the binary today and neither document claims it does.

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

Fifteen commands, near half the current CLI, that a general user never meets.
It stays a deep noun group and is never promoted.

```
zeb k8s cluster init|describe|validate|set-image|set-replicas|…
```

## 3a. What configures a server mode

Group 1 takes no nouns and no flags, so its whole input is the environment. That
made the variables a flat list nobody could hold: `zeb help` used to name eight
of the twenty-two below, in one block, in no order.

They are grouped here by **what a variable decides**, because that is the
question an operator arrives with. The tables below are the whole operator-facing
set; they were read off `std::env::var` call sites, not remembered, and the
paragraph after them accounts for every name that exists and is not in a table.

**Process shape** — where this process listens and what it writes to.

| Variable | Decides | Default |
| --- | --- | --- |
| `ZEBFLOW_PLATFORM_HOST` | listen host | `127.0.0.1` |
| `ZEBFLOW_PLATFORM_PORT` | listen port | `10610` |
| `ZEBFLOW_PLATFORM_DATA_DIR` | data root | the OS user-data path (§5) |
| `ZEBFLOW_PLATFORM_BASE_URL` | external base URL in OAuth redirect and MCP session URLs | derived from request headers |
| `ZEBFLOW_HEALTH_PORT` | dedicated liveness port | unset: no separate server |
| `ZEBFLOW_HEALTH_HOST` | dedicated liveness host | `ZEBFLOW_PLATFORM_HOST` |

**First-boot bootstrap** — what exists inside the instance after its first start.

| Variable | Decides | Default |
| --- | --- | --- |
| `ZEBFLOW_PLATFORM_DEFAULT_OWNER` | owner account created on first boot | `superadmin` |
| `ZEBFLOW_PLATFORM_DEFAULT_PROJECT` | project created for that owner on first boot | `default` |
| `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD` | that owner's initial password | generated into `<data-dir>/.bootstrap/superadmin-password` |
| `ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD` | permits the literal password `secret` | off |

`DEFAULT_OWNER` and `DEFAULT_PROJECT` are **server** state, not client context.
They name what the server creates on first boot. They are not a record of which
project a person is working on, and reading them as one is the mistake §5
records: `distribution.md` calls them "CLI configuration", and no `zeb config
set` writes them. Nothing consults them to resolve `--owner` / `--project`.

**Cluster membership** — how a controller and an office find and trust each other.

| Variable | Decides | Required by |
| --- | --- | --- |
| `ZEBFLOW_CLUSTER_JOIN_TOKEN` | shared internal cluster token | controller **and** office |
| `ZEBFLOW_CLUSTER_MASTER_URL` | controller base URL an office registers with | office |
| `ZEBFLOW_CLUSTER_ADVERTISE_URL` | base URL this node advertises | optional; defaults to this process's listen URL |
| `ZEBFLOW_CLUSTER_NODE_ID` | stable node id | optional; defaults to the role name |
| `ZEBFLOW_CLUSTER_NODE_LABEL` | human-readable node label | optional; defaults to the node id |

A required variable that is unset **or blank** is missing. An exported-but-empty
variable is a misconfiguration, and treating it as a value is how a blank
advertise URL used to disable office registration without saying so.

**Sessions and tokens.**

| Variable | Decides | Default |
| --- | --- | --- |
| `ZEBFLOW_COOKIE_SECURE` | `Secure` attribute on session cookies | on unless the listen host is loopback |
| `ZEBFLOW_SECRET_ROTATION_EPOCH` | unix timestamp invalidating older platform-issued tokens | `0` |

**Hub.**

| Variable | Decides | Default |
| --- | --- | --- |
| `ZEBFLOW_HUB_DEFAULT_BASE_URL` | default hub API URL | `https://hub.zebflow.com/api` |
| `ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE` | permits localhost hub remotes | off; production must leave it unset |

**Rendering engine**, advanced and rarely set.

| Variable | Decides | Default |
| --- | --- | --- |
| `ZEBFLOW_PLATFORM_RWE_ENGINE_ID` | engine for the platform admin UI | built-in |
| `ZEBFLOW_RWE_ENGINE_ID` | engine for project pipeline rendering | built-in |
| `ZEBFLOW_RWE_PREWARM` | set to `0` to disable post-compile SSR warmup | on |

Every other name in the source is deliberately not interface.
`ZEBFLOW_RWE_DEMO_ENGINE_ID` belongs to the separate `axum_rwe_demo`
binary, `ZEBFLOW_KEEP_DOCSGEN_TEST` and `MAPSERVER_BENCH_SOURCE` are test-only,
and `RWE_WORKER_COUNT`, `RWE_SSR_CACHE_TTL_SECS`, and
`PIPELINE_NODE_TIMEOUT_SECS` are unprefixed engine tuning knobs that should
either take the `ZEBFLOW_` prefix and join the tables above or stop being
environment-configurable. `ZEBFLOW_PORT` is a stray: its only effect is the
fallback base URL of one WASM lifecycle callback, where it defaults to `10611`
rather than the real listen port, and it should be deleted in favour of
`ZEBFLOW_PLATFORM_BASE_URL`. `ZEBTUNE_LLM_PROVIDER` and its five
`ZEBTUNE_OPENAI_*` / `ZEBTUNE_ANTHROPIC_*` companions are a fallback LLM
credential for the agent node, read inside the server but belonging to the
credential surface rather than to a server mode.

## 4. Aliases

An alias is permitted only when it names one canonical command exactly. The
alias is sugar; the canonical form is the vocabulary.

| Alias | Expands to |
| --- | --- |
| `zeb install <ref>` | `zeb project install <ref>` |
| `zeb remove <ref>` | `zeb project remove <ref>` |
| `zeb list` | `zeb project list` |

The alias set is frozen at the three rows above. Adding a fourth is a change to
this document. The remaining Group 2 words — `status`, `login`, `use`, and
`logout` — are not aliases and never gain one: they name no canonical longer
form, because §3 explains they configure the client rather than acting on
anything an instance holds.

Group 1's `master` and `worker` are **not** additions to this set. They are
deprecated spellings on their way out, not sugar being kept, and §3's rule that
each names exactly one canonical word is what separates them from the docker
wart §2 rejects.

## 5. There is no current directory

A Zebflow project lives at `<data-root>/users/{owner}/{project}/`. It is owned
by the instance, not by a folder you `cd` into. Nothing about the shell's
working directory identifies a project.

So context is explicit and stated, the way `kubectl` and `gcloud` learned to do
it, rather than inferred from where the user happens to be standing.

**And neither does the working directory identify the instance.** The data root
has exactly two cases:

```text
ZEBFLOW_PLATFORM_DATA_DIR set  ->  use it
otherwise                      ->  the OS user-data path
```

which is `$XDG_DATA_HOME/zebflow` or `~/.local/share/zebflow` on Linux,
`~/Library/Application Support/Zebflow` on macOS, and `%LOCALAPPDATA%\Zebflow`
on Windows. An exported-but-blank variable is unset, the same rule §3a applies
to the cluster variables.

There is no third case, and in particular none that consults the working
directory. The default used to be `.zebflow-platform-data` relative to it, which
contradicted the paragraph above in the most direct way available: an installed
binary created an instance wherever the user happened to be standing, and a `cd`
lost the projects in it. **The contents of the directory do not change between
the two cases.** `platform/`, `services/`, `users/{owner}/{project}/{repo,data,
files}`, and `.bootstrap/` are identical either way, because both cases produce
a `PathBuf` that goes into the same layout code and nothing downstream branches
on which one produced it — `platform::boot` holds the one function that decides,
and a test asserts the two trees are equal.

This repository's own `dev.sh` sets the variable, like every other piece of
explicit configuration in it. A development server sharing a data root with an
installed `zebflow` would be the same conflation from the other side.

**First use provisions itself.** `zeb install <ref>` on a machine with no
instance at the resolved data root creates one, creates the superadmin account,
writes the generated password 0600 as first boot already does, and then reads
that file and logs in **normally** — the same `POST /login`, the same session
token, the same context store below. Reading a file that is mode 0600 and owned
by the same user *is* the check; no second authentication mechanism exists, and
a file that cannot be read produces a login failure, which is the right outcome.
The command prints where the instance is and where its password is, and the
person does not have to go and read that file to continue, because the sign-in
already happened.

The file's existence is the zero-ceremony window, and it closes: the first
successful password change (the web UI's change-password screen, backed by
`POST /api/profile/password`) deletes it. From then on the self-login above
fails with an error naming `zeb login` — one explicit sign-in, after which the
stored session token carries every later command, the gh/docker pattern.
`zeb admin reset-password <owner>` is the offline recovery: it rotates the
credential in the catalog directly and prints it once, without recreating the
file.

```
zeb login http://localhost:10610
zeb use superadmin/default
zeb status
```

**Resolution is explicit, and nothing guesses:**

```text
--owner / --project flag  →  stored client default  →  error
--instance flag           →  stored client default  →  this machine
```

The last step of the second line is the one addition, and it is not an
inference from anywhere. With no flag and no stored context there is exactly one
instance a person can be talking about — the one on the machine they are typing
on, at the data root above — and naming it is what lets a cold machine install
something without a `login` first. It reads nothing about the working directory,
which is what the rest of this section forbids. A stored instance still wins
over it and a flag over both, so nothing that was stated becomes a guess, and a
*remote* instance that does not answer is an error rather than a quiet
substitution of a local one.

`distribution.md` says `default_owner` and `default_project` "already exist in
the CLI configuration". They exist, but as *server* configuration read from
`ZEBFLOW_PLATFORM_DEFAULT_OWNER` and `ZEBFLOW_PLATFORM_DEFAULT_PROJECT`, which
name the owner and project the server bootstraps on first boot. They are not a
client-side record of which project a person is working on, and nothing consults
them to resolve `--owner` / `--project`.

**The client context store is separate, and is what `login`, `use`, and `logout`
write.** It records four things — instance URL, credential, owner, project — in
`~/.zebflow/client/context.json`, with the directory `0700` and the file `0600`,
the same shape first-boot bootstrap uses for a generated password.

What it stores as the credential is the **session token** the server issued, not
the password that obtained it. A token expires, is revoked by `zeb logout`, which ends the session on the server
before forgetting it locally, and
buys whoever reads the file one account's session rather than a password that
was probably reused elsewhere. A context file readable by anyone but its owner
is reported on stderr and tightened on the next write, rather than refused:
refusing to read it would stop the `zeb logout` that revokes what leaked.


Project context is **not** defaulted, because bare `zeb install` does not consult
it (§7). `zeb install kids-educational-games` needs no setup for that reason, not
because an unset project is filled in — the command materialises a project named
by the reference, and the review prints before anything is written. The instance
is a different field and is defaulted, for the reason just given; owner and
project are not.

Context matters for the scoped forms, `zeb project install` inside an existing
project among them, where it is stated rather than inferred.

## 6. Transport

**HTTP when a server owns the state, direct when none does.**

That is one rule over three states, where the previous wording — "direct when
the command exists to repair a server that will not start" — was one rule over
two and left the third unnamed:

```text
a server is listening        ->  HTTP to it
nothing is listening         ->  open the data root in this process, and exit
Group 3 maintenance          ->  direct, because the server will not start
```

The repair commands and a cold machine are the same case under the restated
rule: in neither does a server own the state.

The reason is not taste. An install touches ten things beyond writing files,
including `node_registry.refresh_project`, which is an in-memory cache inside the
running server. A CLI that wrote to disk behind a live server would leave it
believing something false about its own nodes. **That argument holds only while
a server is running.** With none, there is nothing to keep coherent, and
starting a background daemon so that the client has something to talk to would
be a process nobody asked for — §3 says which four words exist for asking.

Groups 2 and 4 therefore speak to a running instance over its API — the same
routes the web UI uses, which is what keeps the two surfaces honest — and reach
the same router in-process when there is no instance to speak to. It is the same
router either way, so a command cannot behave differently depending on which
transport carried it. In-process means in-process: no socket is bound, which
also keeps `/login` from being exposed to every other process on the machine for
the length of an install. Group 3 runs directly against the data directory.

An unreachable *remote* instance is an error, not a reason to fall back. Only
this machine's own instance is opened directly, because it is the only one a
client can open at all.

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

`zeb remove` is not the inverse. It deletes a project, which is the only
removal the platform has at this scope, and it says so rather than presenting
itself as an undo. Project-scope uninstall — the one that does remove a tracked
dependency — exists as an API for node bundles and has no CLI verb.

## 8. The mapping, and what it does not cover

This section was a promise. It is now the result of running it.

The vocabulary was checked against every operation the binary actually
performs. The counts below were extracted from the code, not remembered: the
`.route(` calls in `src/platform/web/mod.rs`, the `#[tool(` attributes in
`src/platform/mcp/handler.rs`, and the dispatch in `src/bin/zebflow.rs`,
`src/platform/cli/`, and `src/provision/k8s.rs`.

`zeb run` was the first collision this mapping had to resolve and the only one
already settled: it materialised a project from a hub asset URL and then served
it, which was half Group 2 and half Group 1. It now only serves, and only what
is already installed, so it sits wholly in Group 1 (§3). Everything below is
what the rest of the pass found.

### 8.1 What exists

| Surface | Verified | This section previously said |
| --- | --- | --- |
| HTTP route registrations | **222** | 222 |
| HTTP method handlers | **264** | not counted |
| Person-facing HTTP operations | **184** | not counted |
| MCP tools | **37** | 37 |
| CLI commands | **31** | 27 |

The route number was right by accident and wrong as a unit. An earlier count
grepped path string literals, which catches `format!()` calls that *build* a
URL, doc examples, and test fixtures alongside real registrations. Forty-four of
the unique `"/api/…"` literals in the Rust source name no registered route at
all, among them
`/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}/activate` — a
route shape deleted when `file_rel_path` became a pipeline's only identifier.
There are also, separately, 222 `.route(` calls. The sets are not the same set.
The per-noun figures that count produced were not a coincidence and were simply
wrong: `pipelines 28` against eleven registrations, `templates 24` against ten.

**One `.route(` is not one operation.** Thirty-six of the 222 registrations
carry more than one method, adding forty-two handlers;
`/api/projects/{owner}/{project}/mcp/session` alone is four. The unit that can
be mapped to a word is the method handler, and there are 264 of them.

`31` CLI commands is four canonical modes (`standalone`, `controller`,
`office`, `run`), seven client verbs, three `project … migrate` commands,
`help` and `version`, and fifteen `k8s cluster` commands — counting an alias
and its expansion once, and the deprecated `master` / `worker` spellings not at
all. Two corrections fall out of that: §3 called Group 4 "ten commands, a third
of the current CLI" when it is fifteen of thirty-one, and now says so; and
`help` and `version` are top-level words belonging to none of §3's four groups,
which claims every command belongs to exactly one. The second is left standing,
because deciding where those two words go is a change to the groups.

### 8.2 What is excluded, and why

Eighty of the 264 method handlers are not operations a person invokes by name.
Each group is listed with its reason, because an unexplained omission is how a
coverage claim becomes false.

| Group | Handlers | Why not an operation |
| --- | --- | --- |
| Page routes | 26 | A person navigates to these; they render HTML and change nothing. Every one is a `GET`. |
| Asset and object serving | 17 | Favicons, branding, platform and project assets, compiled RWE scripts, library files, ZebFS object reads, node icons, hub publisher media. These serve bytes at a URL. |
| Public ingress and delivery | 9 | `/wh/*`, `/ms/*`, the WebSocket room and preview sockets, and the debug reload stream. A person triggers these by using an application, not by naming a command. |
| Internal machine-to-machine | 8 | All of `/api/internal/*`: cluster register and heartbeat, runtime materialize, execute, and webhook forwarding, and project-transfer internals. Authenticated by the cluster token, never by a session. |
| Remote-hub far end | 9 | `/api/hub/remote/*` plus the four `artifact` / `artifacts/{sha256}` byte fetches. These answer *another* instance holding a publisher token or a digest. Nothing in this instance's own UI or CLI calls them; they are the server half of an operation whose client half is counted under `hub`. `distribution.md` §2 names the byte fetch `artifact()`, one of three calls behind the repository interface, and it is the one no person makes. |
| Second spellings | 6 | One operation reachable two ways: `POST /home/projects/create` is the form twin of `POST /api/users/{owner}/projects`; `POST …/files/access` of the `PUT`; `PUT …/credentials/{id}` of `POST …/credentials`; `PUT …/db/connections/{slug}` of `POST …/db/connections`; `PUT …/docs/file` of `POST …/docs`; and `GET …/hub/assets/preview` is literally the same handler as `GET …/hub/publish-preview`. |
| Liveness and readiness | 2 | `/health` and `/ready`. Probes. The three routes on the dedicated health server are a separate router and are not in the 222 at all. |
| Protocol discovery | 3 | The two `/.well-known/oauth-*` documents and `/oauth/callback`. Transport for an OAuth exchange a person starts elsewhere. |

`POST /login` and `POST /logout` are **not** excluded. They are session
operations with CLI counterparts, and §8.4 records that one of those
counterparts does not actually call its route.

### 8.3 The mapping

184 operations, grouped by the noun that would reach them. The nouns are read
off what the operations do, not off the route path — §8.5 says where those two
disagree.

| Noun | Ops | Reached by a built term | Named, not built | Uncovered |
| --- | --- | --- | --- | --- |
| `hub` | 49 | 2 | 3 | 44 |
| `source` | 20 | — | — | 20 |
| `db` | 20 | — | — | 20 |
| `project` | 20 | 2 | 1 | 17 |
| `pipeline` | 12 | — | — | 12 |
| `docs` | 10 | — | — | 10 |
| `git` | 8 | — | — | 8 |
| `instance` | 7 | — | — | 7 |
| `credential` | 6 | — | — | 6 |
| `node` | 5 | — | 3 | 2 |
| `file` | 5 | — | — | 5 |
| `mcp` | 5 | — | — | 5 |
| `account` | 4 | — | — | 4 |
| `mapserver` | 4 | — | — | 4 |
| `lib` | 3 | — | 1 | 2 |
| `assistant` | 3 | — | — | 3 |
| `session` | 2 | 1 | — | 1 |
| `help` | 1 | — | — | 1 |
| | **184** | **5** | **8** | **171** |

**Five operations are reachable by a word that exists and works.** `zeb list`
reaches `GET /api/users/{owner}/projects` and `zeb remove` reaches
`DELETE /api/users/{owner}/projects/{project}`, which are the two under
`project`. `zeb install` reaches `POST /api/platform/hub/install` and its
`…/review` sibling, which are counted under `hub` because that is whose routes
they are — the one Group 2 verb that creates a project is a hub operation, and
the noun the vocabulary gives it is not the noun the route table gives it.
`zeb login` reaches `POST /login`. `zeb status` and `zeb use` are the client
context store and reach nothing on the instance except `GET /health` and
`GET /api/profile`, which they read rather than command.

**Eight are named by `distribution.md` §0a and marked not built**, which is a
different thing from uncovered and is kept separate here:

```text
zeb project export <kind>   → POST …/transfer/export/{kind}
zeb node install <ref>      → POST …/nodes/install  and  …/nodes/install/review
zeb node uninstall <kind>   → DELETE …/nodes/uninstall/{kind}
zeb lib add <ref>           → POST …/rwe/libraries/enable
zeb hub add <ref> --to      → POST …/hub/assets/{id}/{version}/add
zeb hub publish <src> --to  → POST …/hub/assets/publish  and  …/hub/remote/assets/publish
```

**The remaining 171 are covered by no term at all.**

The 37 MCP tools add nothing to that total and change nothing about it: every
one of them is an operation, and not one has a CLI word. Thirty-two are the
agent-facing rendering of routes already counted. Five reach behaviour no HTTP
route offers, and so are operations the API surface does not contain:
`pipeline_run` executes a node body ephemerally with nothing saved, logged, or
counted as a hit; `pipeline_search` and `template_deps` have no route at all;
and `start_here` and `help_search` are entry points into the help corpus that
`GET …/help` returns whole. A sixth is partial: `git_command` permits `log` and
`diff`, which the six `git/*` routes do not.

### 8.4 The gap list

This is the deliverable. Without it, "covers all current operations" is a claim
nobody can check.

**`project` — 17 uncovered.** `install` creates a project from a hub reference
and `remove` destroys one. Nothing in the vocabulary touches a project that
already exists. Uncovered: create an empty project
(`POST /api/users/{owner}/projects` — a project can be made without a package,
and no word says so); clone a project from a git remote
(`POST /home/projects/clone`, which creates a project and is the only route
that does so outside the hub install path); change a project's owner
(`POST …/transfer/owner`); import a transfer archive; download a completed
export; list transfer operations; read and write any settings section; read and
clear the invocation log; read runtime status; sync the runtime; toggle
preview; read preview status; read dependency status; repair dependencies;
reindex.

**`hub` — 44 uncovered**, more than double the next largest, and it is four
surfaces wearing one name. *Assets*: publish, publish-review, retract,
presentation, add, review, browse local, browse remote, browse mine, list
publish sources, preview a publish source. *Repositories*: create, list, and
delete — at **three** scopes, `/api/projects/{owner}/{project}/hub/repositories`,
`/api/users/{owner}/hub/repositories`, and `/api/platform/hub/repositories`,
which are three route families for one concept. *Identity*: publishers, tokens,
and grants, each with create, list, and delete. *Service*: producer mode, and
the hub service configuration that decides whether this instance serves a hub
at all. `zeb hub add` covers one route and `zeb hub publish <source> --to <hub>`
covers two, local and remote, depending on what `<hub>` names; the other
forty-four are unnamed.

**`source` — 20 uncovered.** Template workspace, search, page list, file read,
save, delete, outline, create, move, git-status, diagnostics, lock-toggle;
project asset list, upload, delete; RWE compile-cache clear; the editor
completion catalog; and the UI component catalog's list, review, and install.

**`db` — 20 uncovered.** Connection create, read, update, delete, and test;
describe, schemas, tables, functions, query, table-preview; the sekejap store's
table create, update, delete, and list; schema export and sync; maintenance
health, sync, and compact.

**`pipeline` — 12 uncovered.** Registry, list, get-by-id, upsert, delete,
lock-toggle, activate, deactivate, execute, DSL, hits, invocations. §2 rejects
`kubectl`'s verb-first model on the grounds that `activate` and `execute` do
not collapse into get/create/delete. They do not, they exist, and neither has a
word.

**`docs` — 10 uncovered.** Project docs list, read, write, delete, folder
create, move, entry delete; agent docs list, read, write.

**`git` — 8 uncovered.** Status, health, repair, commit, remote read, remote
set, branch list, branch checkout.

**`instance` — 7 uncovered.** `GET /api/meta`, `GET /api/system/info`, the four
admin database routes (collection list, query, node read, node delete), and
`GET /api/cluster/workers`. `zeb status` reports the client's stored context
and reads `/health`; it reports nothing an instance knows about itself.

**`credential` — 6 uncovered.** Type list, list, upsert, read, delete, and
OAuth authorize. `distribution.md` §1 rules that credential *values* are never
distributed; managing them is a separate act and has no word either.

**`node` — 2 uncovered** beyond the three named-not-built: list node
definitions, and read one by kind.

**`file` — 5 uncovered.** List, mkdir, upload, remove, set access.

**`mcp` — 5 uncovered.** Session read, create, toggle, revoke, reset token.
The instance can mint an agent credential and the vocabulary cannot say so.

**`account` — 4 uncovered.** List users, create a user, read profile, update
profile. Every operation in this document is scoped by an owner and there is no
word for one.

**`mapserver` — 4 uncovered.** Source list, layer list, layer publish, layer
delete.

**`lib` — 2 uncovered** beyond `zeb lib add`: list available libraries, and
disable an enabled one. `add` is named with no removal counterpart, while the
route exists.

**`assistant` — 3 uncovered.** Config read, config write, chat.

**`session` — 1 uncovered, and it is a stated behaviour the code does not
perform.** `POST /logout` removes the server-side session and clears the
cookie. `zeb logout` clears `~/.zebflow/client/context.json` and calls nothing.
§5 says the stored token "is revoked by `zeb logout`, which ends the session on the server
before forgetting it locally"; it is forgotten, not
revoked, and the session stays valid on the instance until it expires. Either
`zeb logout` calls the route or §5 stops claiming a revocation.

**`help` — 1 uncovered.** `GET …/help` returns the project help corpus.
`zeb help` prints the CLI's own usage. One word, two unrelated things.

### 8.5 The route-table nouns, and which collapses hold

The list this section used to carry was produced by counting path segments. It
proposed five collapses. Three survive contact with the code and two do not.
Each is recorded with its evidence so none is re-proposed.

**`tables` into `db` — correct, and the evidence is stronger than the guess.**
`…/tables/*` is the sekejap store's schema surface and
`…/db/sekejap/maintenance/*` is the same store's health surface. One store
already has routes on both sides of the split, which is not a design.

**`assets` into source — correct.** `api_upload_asset` writes to
`layout.repo_assets_dir()` and requires `ProjectCapability::TemplatesWrite`.
By `distribution.md` §6, `repo/` is what the human declares. Same area, same
capability, same noun.

**`transfer` into `project` — correct.** Export, import, download, operation
list, and owner change all act on a whole project, and `distribution.md` §0a
already spells the first of them `zeb project export <kind>`.

**`rwe` into source — wrong.** `POST …/rwe/libraries/enable` resolves a lock
entry and writes it through `dependency_lock.enable_rwe_library`. That is
dependency management, not source: `distribution.md` §1 classifies an RWE
library as a distributable resource with `RweLibraryManifest` as its kind and
`DependencyLock` as its record, and §0a already names the verb `zeb lib add`.
`lib` is the noun, and only `…/rwe/cache/clear` — which evicts compiled
templates — belongs with source.

**`git` into `project` — wrong.** `distribution.md` §2 lists a git remote as a
**channel**, beside Hub asset, local file, and transfer archive, each with its
own trust story. The routes carry git's own verb names, the MCP surface exposes
`git_command` as a subcommand passthrough, and `zeb project commit` would say
neither that it commits nor that it commits `repo/`. The one git-shaped
operation that *is* a project act is `POST /home/projects/clone`, which creates
a project and belongs under `project` for that reason — which is exactly the
evidence that the two are separable.

**The reduction this section hoped for does not exist.** Correcting the five
collapses leaves eighteen nouns, the same number it started with, because the
old list was never eighteen nouns: it enumerated seventeen despite claiming
eighteen, five of its entries (`settings`, `transfer`, `rwe`, `tables`,
`assets`) were paths inside other nouns, and it omitted five groups the route
table does have (`account`, `instance`, `session`, `help`, and the UI catalog).
Renaming does not shrink the surface.

The reduction has to come from a different decision: **which operations are
interface at all.** Six nouns — `project`, `hub`, `node`, `lib`, `source`,
`pipeline` — carry everything a person obtains, publishes, or authors, and are
the plausible terminal vocabulary. The other twelve are administration console
surfaces that a terminal may never need. A vocabulary that is minimal and
complete does not have to cover 184 operations; it has to state which ones it
declines to cover, and why. That statement does not exist yet, and it is the
next piece of work, not a thing this survey may decide.

### 8.6 Proposed nouns

Proposed, not added. §1 says nouns grow and verbs freeze, and a noun a survey
found is still a term nobody agreed. None of these is in the vocabulary until
it is added here deliberately.

```
project · hub · node · lib · source · pipeline
db · file · credential · docs · git · mcp
assistant · mapserver · account · instance · session · help
```

Four of these already appear in a shipped document and are the least
contentious: `project` is built, and `hub`, `node`, and `lib` are named in
`distribution.md` §0a. `source` is the name §8 used informally for the
`templates` / `assets` collapse and has never been proposed as a word.
`account`, `instance`, `session`, and `help` are new here and were found by
this survey, not designed.

The verbs are a separate and larger problem. §2 rejected uniform CRUD because
`publish`, `activate`, `migrate`, `review`, and `retract` do not collapse into
it. All five exist as operations. So do twenty-two more the survey turned up —
`add`, `checkout`, `commit`, `compact`, `deactivate`, `delete`, `disable`,
`enable`, `execute`, `export`, `get`, `import`, `move`, `query`, `reindex`,
`repair`, `search`, `set`, `sync`, `test`, `toggle`, `upload` — against a
frozen set of nine outside Group 4. §1 says a verb is permanent once typed.
Freezing twenty-seven at once is not a survey's decision, and this document is
where it would be made.
