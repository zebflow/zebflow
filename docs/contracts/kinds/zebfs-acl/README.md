# ZebFsAcl

Status: **review** — spec settled 2026-08-29; code caught up. Audited 2026-09-01: longest match wins, an unmatched path is private (`ZebFsAccess::default`), the reserved prefix is refused, unknown fields are refused, and the pre-contract bare shape is accepted on read and rewritten enveloped. The entries under Open are open, not owed.

Which of a project's files the public may read. One document per project.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `ZebFsAcl` |
| Document | `.zebfs/acl.json` in the project's store: `files/.zebfs/acl.json` on disk, or that key under the project's prefix in its bucket |
| Reserved prefix | `.zebfs/` — the folder holding this document; never an object |
| Adapter | `src/zebfs/acl.rs` |
| Backend-neutral | the same document describes local files and future object stores |

## Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "ZebFsAcl",
  "metadata": { "name": "joseph/myblog" },
  "spec": {
    "rules": {
      "uploads/photos": {
        "access": "public_read",
        "scope": "prefix",
        "updated_at": 1782287499
      },
      "uploads/photos/passport.jpg": {
        "access": "private",
        "scope": "object",
        "updated_at": 1782287500
      }
    }
  }
}
```

The photos folder is public; that one passport picture is not.

| Field | Rule |
| --- | --- |
| key | the object or prefix path the rule covers, normalised, no leading slash |
| `access` | `private` or `public_read`. Nothing else exists |
| `scope` | `object` — that path only — or `prefix` — that path and everything beneath it |
| `updated_at` | unix seconds, when the rule was last set |

## Resolution

- **Longest match wins.** A `prefix` rule on `uploads` and an `object` rule on
  `uploads/passport.jpg` leave the passport private and the rest public.
- **No matching rule means private.** Default deny.
- **A missing or unreadable document means everything is private.** A
  permissions file may only fail in the safe direction.

## Why it carries an envelope

It is a document written to disk, so it looks like every other document written
to disk. The contrast is [`FileRef`](../file-ref/README.md), which stays bare
because it is a value inside a running pipeline and is never written on its own.

Unlike [`DependencyLock`](../dependency-lock/README.md), **this document cannot
be regenerated.** A lock is derived from requested state; an ACL records a human
decision — *I chose to share this folder*. Deleting it makes every file private
again, which is safe, but the intent is gone and nothing can rebuild it.

That is why the pre-release allowance here is different from the lock's: a bare
`{ "rules": … }` document, the shape written before this contract, is **accepted
on read** and rewritten in the enveloped form on the next change. Nobody loses a
sharing setting to a format change.

## Rejections

Unknown root or spec fields. Unknown `access` or `scope` values. A rule key that
escapes the project's files (`..`, absolute paths) or that addresses `.zebfs/`.
A future `apiVersion`.

## Open

- **Who may change a rule.** The document records the decision, not the
  authority behind it. Which roles may publish a file is not stated here or
  anywhere else.
- **Deletion.** Whether removing an object removes its rule, or leaves an
  orphan rule that would apply again if a file of the same name reappeared.
- **Listing.** Whether a public prefix implies a public listing of its
  contents, or only direct access to objects whose paths are already known.
