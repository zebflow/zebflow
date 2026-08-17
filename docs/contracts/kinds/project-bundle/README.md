# ProjectBundle

Status: pending review

This contract defines project export and import archives. Its review will cover
archive layout, included data classes, checksums, path safety, staged import,
rollback, and full operational recovery boundaries.

## Recorded before review

### Intra-project references can dangle

Found while reviewing `NodeBundle`, recorded here because it is this contract's
concern rather than that one's.

`n.function.call` invokes another pipeline in the same project by slug, and
`n.trigger.function` is the entry point it targets. Nothing verifies that the
target resolves. A project copied without the called pipeline therefore carries
a reference to something that does not exist, and the failure appears only at
run time.

This is the same class of problem as a copied Python project missing a file.
It is distinct from a missing node implementation:

| Missing thing | Obtainable? | Described by |
| --- | --- | --- |
| node implementation | yes — install the package, or scaffold from its interface | `repo/nodes/{kind}.json` |
| function pipeline | **no** — it should have travelled with the project | nothing today |

A missing node can be repaired. A missing function pipeline cannot be fetched
from anywhere, so the honest response is to report precisely what is absent
rather than to offer a repair that does not exist. ProjectBundle therefore has
to decide which intra-project references an export must carry, and import has to
verify them.

### Detection belongs to one report

`DependencyLockService::status` already reports four families: `rwe_library`,
`node_bundle`, `node_kind`, and `pipeline_source`. Unresolved function calls
should become a fifth family there rather than a separate mechanism, so a
project has one answer to "what is missing".

That report is served by `GET /api/projects/{owner}/{project}/dependencies` but
is not consulted when a pipeline is opened, which is why an author currently
meets a broken node before meeting the explanation. Surfacing it on open is a UI
decision, recorded here so it is not rediscovered.
