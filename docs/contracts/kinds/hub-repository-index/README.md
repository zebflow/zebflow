# HubRepositoryIndex

Status: **Pending**

## 1. Purpose and owner

`zebflow-repository.json` is what a **static repository** serves at its base
URL. It says which packages that location offers, which releases of each it
holds, where each release document sits, and which bytes are the right ones.

Owner: `platform`. Boundary: transferred.

It exists because the static repository channel in
[`distribution.md`](../../distribution.md#static-repositories) is a plain HTTPS
location with no server logic. Nothing there can answer a query, so the answer
has to be a file, and a reader must never guess a path.

**This is the only format a static repository itself owns.** Everything it
points at is a [`HubPackage`](../hub-package/README.md) document, which already
exists and is unchanged. A repository is a transport, not a format.

Anyone can publish one, so it is treated as a durable public format: versioned
by `apiVersion`, strictly decoded, and explicit about what a reader refuses.

## 2. Representation

Canonical `apiVersion` / `kind` / `metadata` / `spec` envelope.

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "HubRepositoryIndex",
  "metadata": {
    "name": "zebflow-hub"
  },
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
          },
          {
            "version": "0.9.0",
            "path": "packages/zebflow.kids-educational-games/0.9.0/package.json",
            "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "size_bytes": 240
          }
        ]
      }
    ]
  }
}
```

The directory it describes:

```text
https://<host>/<path>/
  zebflow-repository.json
  packages/<package-id>/<version>/package.json
  artifacts/<sha256>
```

`artifacts/` is the same content-addressed layout every other channel uses, so
a package that references bytes rather than carrying them installs from a static
repository as it does from a hub.

## 3. Boundaries

| Boundary | Behaviour |
| --- | --- |
| Disk | none. Zebflow never writes this document; it only reads other people's. |
| Database | none. |
| Transfer | one HTTPS `GET` of `<base>/zebflow-repository.json`. |
| Runtime | decoded per fetch. Not cached: an index is the mutable half of a repository and a stale one would resolve to a release that has been withdrawn. |

## 4. Required fields and extension points

| Field | Required | Meaning |
| --- | --- | --- |
| `metadata.name` | yes | what this repository calls itself. Display only. |
| `spec.packages[].package_id` | yes | the package identity, `{publisher}.{slug}` by convention |
| `spec.packages[].asset_kind` | yes | listing hint. **Advisory** — see below |
| `spec.packages[].title` | yes | what it is called |
| `spec.packages[].description` | no | one line saying what it does |
| `spec.packages[].latest_version` | yes | what a reference with no version resolves to |
| `spec.packages[].releases[].version` | yes | the release version |
| `spec.packages[].releases[].path` | yes | where its document is, relative to the base |
| `spec.packages[].releases[].sha256` | yes | the digest of that document's bytes |
| `spec.packages[].releases[].size_bytes` | yes | its size, so a reader bounds the read |

`asset_kind` is **advisory**. The fetched `HubPackage` carries its own, and that
one decides what the install does. An index that misdescribes a package changes
what a listing shows and nothing else — which is the property that keeps the
index out of the trust path for behaviour.

There is **no extension point**. `deny_unknown_fields` applies to every struct,
so an unrecognised field is refused rather than ignored. Adding one is therefore
a new `apiVersion`, which is deliberate: a reader that silently ignored fields
would let a publisher believe it had said something the reader never heard.

## 5. What a reader must reject

The envelope machinery refuses, before any of this:

- an `apiVersion` other than `zebflow.com/v1`
- a `kind` other than `HubRepositoryIndex`
- any unknown field, at any depth
- an empty `metadata.name`

The kind's own validator then refuses:

| Rejection | Why |
| --- | --- |
| over 8 MB | the index is fetched before anything is chosen, so it is the one download a repository can force. It carries no package content, so it has no reason to be large. |
| over 4096 packages, or over 512 releases in one package | bounds a listing walk |
| a `package_id` outside `[a-z0-9._-]`, or starting/ending with `.` | the same id becomes a project slug at install and a key in a client. A name that could hold a separator would be a path in one of those places. |
| the same `package_id` twice | an index that offers one id twice cannot say which it means, and a reader taking the first would resolve by file order |
| an `asset_kind` outside `[a-z0-9_]` | it is a bare token, not free text |
| a package with no releases | it offers nothing |
| the same `version` twice in one package | same reason as a duplicate id |
| a `latest_version` naming no release in `releases` | a bare reference would point at nothing |
| a `version` containing `/` or `\` or a control character | a version is an opaque token and never a path segment |
| a `path` that is absolute, contains `..`, `.`, `\`, or `//` | the path is joined onto a URL the user named. Only descending is allowed, or an index could reach outside the location that was trusted. |
| a `sha256` that is not 64 lowercase hex digits | it is the release's identity |
| a `size_bytes` of 0, or over the 25 MB `HubPackage` ceiling | a document that cannot exist |

The fetcher then refuses, outside the contract:

| Rejection | Why |
| --- | --- |
| a document whose bytes hash to anything but `sha256` | the index pins the release; a file replaced under a path fails closed |
| a document whose `metadata.name` is not the `package_id` that pointed at it | an index must not aim one name at another package's bytes |
| a document whose `metadata.version` is not the `version` that pointed at it | same |
| a redirect | the base URL is the whole trust decision and a redirect moves it |
| a body over the declared size | a repository must not stream something unbounded |

## 6. Concurrency, atomicity, recovery

None of Zebflow's. The document is read whole in one request and either decodes
or does not; there is no partial state. A publisher updating a repository is
responsible for writing the release documents before the index that names them,
because a reader that fetches the index first will otherwise resolve a path that
does not exist yet — which fails cleanly, and is why that ordering is a
convention and not a correctness requirement.

## 7. Security

The index is **not** in the trust path for behaviour. It decides where a reader
looks and which bytes are acceptable there, and nothing else:

- the review that decides whether a package may install reads the fetched
  `HubPackage`, not the index
- `asset_kind` in the index is advisory; the document's is authoritative
- a package from a static repository runs the same review, the same violation
  refusals, and the same digest verification as one from any other channel —
  `distribution.md` §3

It carries no secret, and there is no field a secret could be put in. A private
static repository is one behind an HTTPS location with its own access control;
the read token configured on the source is sent as a bearer header and is never
part of this document.

## 8. Compatibility

`apiVersion` is the version. `zebflow.com/v1` is the only one accepted, and
because every struct denies unknown fields, **any added field is a new version**.
There is no forward-compatible relaxation and none is planned: a repository
format that quietly ignored what it did not understand would make a publisher's
intent unknowable.

## 9. Implementation

| File | Role |
| --- | --- |
| `src/contracts/kinds/hub_repository_index.rs` | the kind: structs, limits, validation, encode/decode |
| `src/contracts/registry.rs` | registration as `ContractKind::HubRepositoryIndex` |
| `src/platform/services/hub_repository.rs` | the fetcher: index fetch, release resolution, digest and identity checks |
| `tests/fixtures/contracts/hub-repository-index/v1-complete.json` | the reference document |

## 10. Tests

In `src/contracts/kinds/hub_repository_index.rs`:

- the reference index decodes and its `latest_version` resolves
- an unknown field is refused rather than ignored
- a release path may not leave the repository base (`..`, absolute, `\`, `//`)
- a release without a usable digest is refused
- `latest_version` must name a release the index carries
- one id offered twice is refused
- a package with no releases is refused
- another kind's document is not read as an index
- a future `apiVersion` is refused rather than guessed
- the reference index round-trips

In `src/platform/services/hub_repository.rs`:

- a static repository reads its index beside its packages, and its artifacts
  under `artifacts/<sha256>`
- an API hub keeps the release-namespaced artifact path
- an empty kind still means the API hub
- a kind this build does not implement refuses rather than guessing
