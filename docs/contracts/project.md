# Project Contract

This contract is for people and tools that build and run Zebflow projects.

## Main Promise

A project can use documented Zebflow features without knowing the internal Rust
code or worker layout.

## Project Owned Work

A project owns its declared:

- source and project settings
- pipelines, functions, scripts, pages, components, and styles
- database schemas and initial data
- files and generated outputs
- project credentials and access rules
- active runtime copies
- Hub package settings
- invocation history when retention is enabled

## Stable Public Areas

Project work may rely on documented:

- project manifests and lock files
- pipeline graph and DSL meaning
- node kinds, settings, pins, input, output, examples, and errors
- expression roots and run context
- RWE, Zeb React, and Zeb Tailwind behavior
- database query and mutation nodes
- FileRef and ZebFS behavior
- credential kinds and outbound request rules
- map, realtime, scheduler, and trigger behavior
- Hub review, add, publish, and project transfer
- HTTP and MCP project interfaces
- stable error codes

## Same Meaning on Every Surface

A feature must keep the same meaning in the visual editor, DSL, HTTP API, MCP,
and generated UI. The controls may look different, but validation, security,
and runtime behavior must agree.

Native, composite, and WASM nodes use the same public node contract.

## Assistant Knowledge

A project carries three assistant documents. They are not one thing, and the
difference decides where each belongs and who may write it.

| Document | Declares | Written by | Authority | Scope |
| --- | --- | --- | --- | --- |
| `AGENTS.md` | the project's agents, their tools, their constraints | a human, or the assistant **when asked** | rules the assistant follows | the project |
| `SOUL.md` | assistant personality, style, and tone | a human | rules the assistant follows | the project |
| `MEMORY.md` | what an assistant noticed while working | the assistant, freely | **none — background only** | **one user** |

### Memory has no authority until a human promotes it

An assistant may notice anything. Nothing it notices governs the project until a
human says so, and that promotion is an edit to `AGENTS.md`: a change with a
diff, a reviewer, and a history.

This has to be real rather than declared. If memory is loaded as instruction,
calling it "background" is laundering, and unconsented text still steers
behaviour. Memory informs; only the authored documents instruct.

### Memory belongs to a user, not to a project

Project-shared memory written without consent is worse than personal memory,
because one person's session silently shapes every colleague's assistant with no
attribution and no review. Assistant memory is therefore scoped to the person
who produced it.

`AGENTS.md` and `SOUL.md` are the opposite: they are the project's declared
rules, shared deliberately, and every member should see the same ones.

### What this requires

Two placements follow from the above and neither is what exists today.

`AGENTS.md` and `SOUL.md` are authored declarations, so they belong under `repo/`
with the project's other source, where git records how a project's rules
changed. They currently sit under `data/cache/agent_docs/`
(`instance-directory.md`, the "Open" item on assistant docs), which is now explicitly the tier that is safe to
delete at any time because nothing reads a deletion as data loss — a stronger
statement of the same defect this section already named.

`MEMORY.md` is correctly derived and correctly under `data/`, but it is keyed by
the project owner rather than by the person working. A project has members
beyond its owner, so the current path cannot express per-user memory at all.

Memory also accumulates without bound. A file gives no eviction and no
retrieval, which are the properties memory actually needs. Whether it stays a
document or becomes a record is open, and is the same question
[`InvocationRecord`](./kinds/invocation-record/README.md) raises about bounded,
disposable history.

## Git

A project's `repo/` is a git repository from birth (`main`). Its remote is
`spec.git.remote` in `zebflow.yaml`: a URL without credentials (`https://`,
`ssh://`, `git@host:path`, or `file://` for a bare repository on this
machine), a branch, and a credential id whose token travels as an
`http.extraheader` on the one command that needs it — never in `.git/config`,
never in a log line. The bare URL is set as `origin`.

Four rules, the same on the Studio panel, the shell (`git …`) and MCP
(`git_command`), because they are one service (`services/git_sync.rs`):

1. **Commit and push are separate acts.** A commit is local and stands on its
   own. Push is explicit, or the `push` flag on a commit, which is
   commit → sync → push and stops at the first step that cannot proceed.
2. **Nothing is lost on a connection error.** Commits the remote has not
   received are counted and shown (`ahead`); the next push carries them.
   `git/status` reports `sync` (branch, ahead, behind, conflicts, dirty) and
   one word: `clean · unpushed · behind · diverged · conflict · unknown · no-remote`.
3. **A conflict is a state, not a dead end.** `sync` fetches and rebases the
   local commits on the remote branch (uncommitted files are set aside and
   restored). A conflict leaves the rebase in progress and names the files;
   each is settled as `mine` (what this project had), `theirs` (the remote's
   version) or by editing the file and marking it resolved; `continue` replays
   the rest and stops at the next conflict; `abort` restores everything.
   A file still carrying conflict markers is refused as a resolution. Git's
   own "ours/theirs" inversion during a rebase never reaches a person.
4. **No force.** Push refuses while behind (`GIT_BEHIND`: sync first) or while
   a rebase is in progress (`GIT_CONFLICT`). History on the remote is only
   ever appended to.

Evidence: `tests/platform/smoke.rs`
`a_git_conflict_is_resolved_in_the_studio_not_locally` walks the whole path
against a bare repository: push, divergence, honest counts, refused push,
conflict kept, mine, continue, push, then theirs and abort.

## Data Movement

Small JSON values may move directly between nodes. Files and large values use a
stable reference or another bounded runtime handle. This internal change must
not surprise downstream code that follows the documented value rules.

Temporary paths are not durable project files. Project code must not require an
undocumented temporary path after the run ends.

## Import and Update Safety

Before adding a package or updating a runtime, Zebflow must report known file
conflicts, executable code, credentials, external URLs, database changes,
public routes, schedules, and format problems. Failure must leave the previous
valid project state available.

Project tools must learn from committed contracts and generated definitions,
not from old conversation history.
