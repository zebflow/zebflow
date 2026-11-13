# ZebFsAcl

Status: **review** — target spec revised 2026-10-02 (three exposure levels,
`serve` whitelist, the document leaves the store; lifetime, matching and
listing settled after the Astra and Fable review the same day). Code caught up
2026-10-03: the document, the three levels, `serve` validation, the store-tier
location, the lifetime rules, the dev file host and `public_execute` on its
origins (`src/platform/web/file_host.rs`), and §Authority for the Studio file
routes (`files.publish`, `updated_by_role`); `/files/…` and `/fs/…` are
deleted. Still owed: a named file host in production. Pipelines write without a
role: kept as the default by the owner on 2026-11-13 (§Open). Mount paths in
`serve` added 2026-11-13 (0.11.28).

Which of a project's files are exposed, and how. One document per project.
Everything it does not name is private.

## Exposure levels

| Level | `access` | Where it answers | Scripts |
| --- | --- | --- | --- |
| 1 | `private` (or no rule) | nowhere without Zebflow sign-in | — |
| 2 | `public_read` | the project's file host | never as a page — every response is sandboxed |
| 3 | `public_execute` | only the origins in `serve` | run |

Every store backend — local ZebFS, SeaweedFS, MinIO, Garage, AWS S3, R2,
custom S3 — stays private at the backend. Exposure exists only in this
document and only through Zebflow's gateway. No backend ever receives a public
bucket policy or an anonymous identity.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `ZebFsAcl` |
| Document | `data/store/zebfs-acl.json` — store tier, beside `addressing.json`; never inside the backend, because a bucket may be written by tools outside Zebflow and whoever writes the bucket must not be able to grant exposure |
| Adapter | `src/zebfs/acl.rs` |
| Backend-neutral | the same document governs every backend |

## Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "ZebFsAcl",
  "metadata": { "name": "demo/blog" },
  "spec": {
    "rules": {
      "covers": { "access": "public_read", "scope": "prefix", "updated_at": 1782287499 },
      "covers/draft.jpg": { "access": "private", "scope": "object", "updated_at": 1782287500 },
      "site": {
        "access": "public_execute", "scope": "prefix", "updated_at": 1782287501,
        "serve": ["https://myblog.example/", "https://www.myblog.example/"]
      }
    }
  }
}
```

| Field | Rule |
| --- | --- |
| key | the object or prefix path the rule covers, normalised, no leading slash |
| `access` | `private`, `public_read` or `public_execute`. Nothing else exists; there is no public write |
| `scope` | `object` — that path only — or `prefix` — that path and everything beneath it |
| `serve` | `public_execute` only, required, non-empty: full origins, no wildcards, each a host the project already registered in [Addressing](../../addressing.md). An entry may end in a mount path (`https://example.com/docs/`): segments of `[a-z0-9-]`, never a first segment of `api`, `assets`, `mcp`, `ms`, `static`, `wh` or `ws`. One rule's entries share one mount path (one build has one base). One address runs one folder: a second rule naming the same host with an overlapping mount (the same path, one inside the other, or the whole host) is refused |
| `updated_at` | unix seconds, when the rule was last set |
| `updated_by_role` | the setter's project role at that moment; absent on rules written before it existed |

## Resolution

- **Longest match wins.** No matching rule means private.
- **A missing or unreadable document means everything is private.**
- `public_read` answers only on the project's file host, always with
  `nosniff` and `Content-Security-Policy: sandbox` — except a PDF, whose
  viewer the sandbox blocks and whose scripts never run on the file host's
  origin. HTML, XHTML, SVG and XML answer as downloads.
- `public_execute` answers unsandboxed only on its `serve` origins. The same
  path on the file host is served as `public_read`; on any other origin it is
  refused.
- A mount path in a `serve` entry gives the folder that path on its host and
  leaves the host's other paths to its pages: the longest mount wins; the path
  without its last slash answers `308` to the mount; a miss under the mount is
  `404`, never a page; only `GET` and `HEAD`; an encoded `/`, `\` or NUL, a
  doubled `/` or a dot segment answers `400`; a service worker script request
  is refused. Scripts there share the host's origin, so they act with that
  host's visitors' sign-in: everyone who can write the folder is trusted that
  far (§Authority). A path mount over an active webhook's literal path is
  refused when set; a webhook registered under a mount later is unreachable
  there. A path mount may stand on a section host (Addressing §2c) and
  answers before its section's routes; a whole-host entry may not.
- A host listed in a `serve` with no mount path answers only that folder, mounted at `/`, with
  `index.html` for a folder path — the same rule as a host carrying a custom
  route ([Addressing](../../addressing.md) §2). It never also answers `pages`.
- The file host is a cookie-less origin of its own, never the Studio's or the
  API's, and never a path on a project host: `{p}.{o}.fs.localhost`, and in
  production a named host with a `files` route ([Addressing](../../addressing.md) §2b).
- Matching is on whole path segments (`site` never covers `site-old`), after
  one normalisation, after every rewrite. The origin compared with `serve` is
  the host the request was routed by, never the `Origin` header.
- A public prefix exposes its objects, not a listing of them.
- A rule change invalidates every cached response it governs.

## Authority

| Act | Who (project role, `access/roles.rs` ladder) |
| --- | --- |
| Set or remove `private` / `public_read` | developer and up |
| Set or remove `public_execute`, or change its `serve` | maintainer and up |
| Write, move or delete beneath a `public_execute` prefix | maintainer and up — it changes a live site |
| Change a rule another member set | the same role as the one stored on the rule, or higher; the owner always |

- A rule set through the Files access API records the setter's role at that
  moment (`updated_by_role`) beside `updated_at`.
- **No pipeline node writes this document.** Generators
  (`web.site.generate`, both modes) write files into a folder,
  private like any upload; exposing that folder is a separate human decision.
- **Exposing a folder trusts every writer of that folder.** The Files page
  says so where the rule is set.
- Exposure is never quiet: the Files list marks every exposed folder and file
  (inherited ones too) — `public_read` amber, `public_execute` red with its
  origins — marks a private folder that contains exposure, and shows exposed
  totals at the top.

## Lifetime

- Deleting an object deletes its `object` rule; a new object of the same name
  starts private.
- Removing a host from Addressing removes it from every `serve`. A rule left
  with an empty `serve` becomes `private` and is flagged in the Files list.
- The document travels with the `store` class of a
  [`ProjectBundle`](../project-bundle/README.md); on import every `serve`
  origin the target has not registered is removed by the same rule.

## Why it carries an envelope

It is a document written to disk. Unlike
[`DependencyLock`](../dependency-lock/README.md) it cannot be regenerated: it
records a human decision. Deleting it makes everything private again — safe,
but the intent is gone. Only the enveloped shape is read; the bare
pre-contract `{ "rules": … }` shape is refused (no backward compatibility
before the first stable release).

## Rejections

Unknown root or spec fields. Unknown `access` or `scope` values. `serve` on a
rule that is not `public_execute`, a missing or empty `serve` on one that is,
an origin with a wildcard, a host the project has not registered, a mount path
with another character, an empty segment or a reserved first segment, entries
of one rule with different mount paths, or an overlapping mount. A
rule key that escapes the project's files (`..`, absolute paths). A future
`apiVersion`.

## Open

- **Whether an MCP tool may change a rule.**
- **What role a pipeline writes with.** Pipelines carry no role today, and
  the owner kept that as the default on 2026-11-13: a pipeline may write,
  move and delete beneath a `public_execute` prefix (Estuary
  `pipeline-writes-beneath-public-execute`).
  Proposed: the activator's role, stored at activation and re-checked on every
  run; a pipeline whose activator lost the role is deactivated with a visible
  reason, and editing it needs a fresh activation.
