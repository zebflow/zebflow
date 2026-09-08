# Pipeline

Status: **Frozen** on 2026-08-15. Amended 2026-09-09 to add optional
`spec.notes`; see [Version Rules](#version-rules).

`Pipeline` is the portable source definition for one executable Zebflow graph.
It is stored in the project repository, can be moved through Git or Hub
packages, and becomes active only after a separate activation operation.

The source contract does not include invocation payloads, credentials, compiled
runtime state, activation timestamps, or invocation history.

## Identity

| Item | Value |
| --- | --- |
| API version | `zebflow.com/v1` |
| Kind | `Pipeline` |
| Canonical files | `repo/{source}/**/*.zf.json`, and `{source}` is the repository itself unless declared |
| Durable representation | Strict UTF-8 JSON |
| Logical identity | `metadata.name`, equal to `spec.id` |
| Project locator | Project-relative `file_rel_path` |
| Contract definition | `src/contracts/kinds/pipeline.rs` |
| Runtime model | `src/pipeline/model.rs` |
| Save and activation service | `src/platform/services/project.rs` |
| Active snapshots | `data/cache/pipelines/**/*.zf.json` |
| Golden fixture | `tests/fixtures/contracts/pipeline/v1-complete.json` |

The logical pipeline id and its project-relative file path serve different
purposes. The id identifies the graph in execution context and logs. The file
path locates one source inside a project and is the key used by the project
catalog and active runtime registry. Moving a file does not silently rewrite the
graph id.

`metadata.version`, `metadata.digest`, and `metadata.annotations` are forbidden.
A pipeline source is mutable project code, not an independently released
package. Hub package versions and digests belong to `HubPackage`.

## Canonical Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "Pipeline",
  "metadata": {
    "name": "catalog-search"
  },
  "spec": {
    "id": "catalog-search",
    "description": "Search the catalog.",
    "metadata": {
      "locked": false,
      "settings": {
        "invocation_retention": {
          "max_invocations": 50,
          "max_age_secs": 604800
        }
      }
    },
    "entry_nodes": ["trigger"],
    "nodes": [
      {
        "id": "trigger",
        "kind": "n.trigger.webhook",
        "input_pins": [],
        "output_pins": ["out"],
        "config": {
          "path": "/catalog/search",
          "method": "POST"
        }
      },
      {
        "id": "respond",
        "kind": "n.web.response",
        "input_pins": ["in"],
        "output_pins": ["out"],
        "config": {
          "status": 200
        }
      }
    ],
    "edges": [
      {
        "from_node": "trigger",
        "from_pin": "out",
        "to_node": "respond",
        "to_pin": "in"
      }
    ],
    "notes": [
      {
        "id": "note1",
        "text": "Create the `oauth2` credential before running this.",
        "x": 120, "y": 40, "width": 320, "height": 140,
        "color": "amber"
      }
    ]
  }
}
```

The golden fixture contains the complete v1 example.

## Field Rules

| Field | Rule |
| --- | --- |
| `spec.id` | Required logical id, equal to `metadata.name` |
| `spec.description` | Optional text, at most 16,384 bytes |
| `spec.metadata.locked` | Prevents editing, moving, activation, and deletion through managed interfaces |
| `spec.metadata.settings.invocation_retention` | Optional override of project invocation retention |
| `spec.metadata.settings.trace_capture` | Optional per-field overrides of project trace capture limits; affects logs only |
| `spec.entry_nodes` | Unique node ids where execution begins |
| `spec.nodes` | Required ordered source list of unique node instances |
| `spec.edges` | Required ordered source list of unique directed connections |
| `node.id` | Unique inside the graph |
| `node.kind` | Registered native, composite, or WASM node kind |
| `node.input_pins` | Input pins exposed by this instance |
| `node.output_pins` | Output pins exposed by this instance |
| `node.config` | Node-owned JSON object validated against its node definition |

Structural objects reject unknown fields. `node.config` is the one intentional
extension point because each node kind owns a different typed configuration.
The pipeline contract requires it to be an object. The node definition and
compiler apply the more specific schema before activation.

Trace capture accepts `array_sample_count`, `max_string_chars`, `max_depth`,
`max_node_bytes`, and `max_run_bytes`. An omitted or null field inherits the
project value, then the engine default. `array_sample_count: 0` keeps all array
elements subject to the independent depth, string, and byte limits. Positive
counts retain the first N elements recursively, including arrays nested inside
objects and retained array elements. Execution data passed to downstream nodes
is unchanged. See the [project defaults](../project-configuration/README.md).

The byte limits apply to captured config/input/output JSON together, excluding
budget-exhaustion markers and trace bookkeeping such as IDs, timings, and error
messages. `max_run_bytes` is per invocation; a called function pipeline uses its
own policy and budget. Count/age retention remains independent of capture size.

Pins are stored on each instance in v1. This supports configuration-dependent
pins such as routes from `n.logic.match`. An edge may use a declared output pin
or the engine-wide `error` output. Its target pin must be declared by the target
instance. Node definitions remain the source used by authoring tools to create
and validate those instance pins.

Cycles are valid. Zebflow pipelines are directed graphs, not limited to DAGs.
The engine applies its bounded execution rules at runtime.

### `spec.notes`

Free-standing annotations drawn on the canvas. Presentation only.

| Field | Rule |
| --- | --- |
| `id` | non-empty, unique among notes; may repeat a node id without collision |
| `text` | markdown, rendered by the editor; empty is legal |
| `x`, `y`, `width`, `height` | canvas geometry, same space node positions use; all default to `0` |
| `color` | a palette name the editor resolves, e.g. `amber`. Free text rather than a closed set, because a note drawn in the wrong colour is a cosmetic problem and refusing to open a pipeline over one is not |

A note never executes, never appears in an edge, and never affects a run. The
reason they exist is distribution: a pipeline installed from a hub must be able
to say which credential to create and which node to change, on the canvas where
the reader already is. Unknown fields inside a note are refused like every other
structural object.

## Frozen Defaults

| Omitted field | v1 meaning |
| --- | --- |
| `spec.description` | No description |
| `spec.metadata` | Unlocked and no per-pipeline settings |
| `spec.entry_nodes` | Runtime derives roots from graph connectivity |
| `node.input_pins` | No input pins |
| `node.output_pins` | No output pins |
| `node.config` | Empty JSON object |

`spec.nodes` and `spec.edges` are always present. Both may be empty only for a
draft. An empty draft is valid source but cannot be activated.

Changing any default meaning requires a new contract API version.

## Validation

The shared contract decoder runs before save, import, installation, activation,
runtime snapshot loading, or runtime synchronization. It rejects:

- missing, unknown, future-version, or wrong-kind envelopes
- unknown fields outside `node.config`
- release metadata on mutable pipeline source
- mismatched `metadata.name` and `spec.id`
- unsupported control characters and unsafe identifiers
- duplicate node ids, entry ids, pins, or edges
- entries or edges that refer to unknown nodes
- edges that refer to undeclared pins
- non-object node configuration
- more than 10,000 nodes or 100,000 edges
- source documents larger than 16 MiB
- invalid invocation retention limits

The stable graph error codes include `FW_ENTRY_NODE`, `FW_EDGE_FROM_NODE`,
`FW_EDGE_TO_NODE`, `FW_EDGE_FROM_PIN`, `FW_EDGE_TO_PIN`,
`FW_DUPLICATE_NODE`, `FW_DUPLICATE_PIN`, and `FW_DUPLICATE_EDGE`.

Contract decoding validates the portable graph. Activation then performs the
runtime compilation preflight, including required node configuration, before
changing active state. A normal pipeline hit uses the already compiled graph and
does not parse or validate the source again.

## Save And Activation

Saving and activating are separate operations:

1. Save decodes and validates the input, then writes canonical JSON to the
   repository.
2. Save updates the project catalog only after the file replacement succeeds.
3. If the catalog update fails, save restores the prior source or removes the
   newly created source.
4. Activation reads the saved source, validates it as executable, checks route
   conflicts, and runs the runtime compilation preflight.
5. Activation writes a content-addressed candidate snapshot.
6. Activation commits by changing the catalog's active hash to that snapshot.
7. If the commit fails, the candidate snapshot is removed and the previous
   active hash and snapshot remain usable.
8. Old snapshots are removed after commit. Cleanup failure cannot roll back or
   corrupt the newly committed active state.

The metadata row is the active pointer. A snapshot without a matching active
pointer is not executable and is safe to remove. An active pointer is written
only after its complete snapshot exists.

Deactivation clears the active pointer before removing snapshots. Runtime
registry refresh uses active snapshots, never mutable draft source.

## Transfer And Generated State

| Boundary | Rule |
| --- | --- |
| Git | Tracks repository `.zf.json` source |
| Hub pipeline or project package | Carries validated source; target project reindexes it through the same save service |
| Composite node bundle | Embedded function pipelines use the same decoder |
| Runtime synchronization | Copies only validated active source and rebuilds runtime registration |
| Active snapshot | Generated from source and selected by catalog active hash |
| Invocation | Uses compiled memory state; no source decoding on each hit |

Active snapshots and catalog metadata are environment state. They are not
committed as project source and can be rebuilt by reactivation.

Pipeline source contains credential ids only. Secret values never belong in
the graph, config examples, runtime snapshots, Hub packages, or Git.

## Version Rules

For `zebflow.com/v1`:

- A patch may fix implementation defects or strengthen rejection of content
  that was already invalid.
- A minor Zebflow release may add node kinds and runtime behavior without
  changing the pipeline source schema or its defaults.
- Adding, removing, renaming, or changing the meaning or default of a pipeline
  field requires a new contract API version and an explicit converter.
- Changing stored pin behavior, identifier rules, size limits in a way that
  rejects valid v1 source, or graph execution meaning requires a new version.
- New fields inside a node's `config` follow that node definition's versioning
  rules. They do not change the Pipeline envelope itself.
- The v1 reader never guesses or silently converts another pipeline shape.

No pre-v1 pipeline format is accepted by the normal reader. A future migration
must use a separate explicit converter, preserve the old source until the new
source reopens successfully, and never run during an ordinary pipeline hit.

Until first release, `zebflow.com/v1` may be amended in place, and every
amendment is recorded below with its date. A converter is not written for a
document that never existed. After first release this section closes and the
rules above apply literally: the schema stops being a draft and starts being a
promise.

### Amendments

| Date | Change | Why it was safe |
| --- | --- | --- |
| 2026-09-09 | Added `spec.notes` | Optional, defaults to empty, and omitted on write when empty -- so a pipeline that never had notes is not modified by being opened and saved, and the golden fixture still round-trips without drift. No existing field changed meaning. It is a sibling of `nodes` rather than a kind inside it, so nothing that executes changed: a note has no pins, no config, cannot be reached by an edge, and no executor learns to skip it. Notes share the id namespace with nothing -- an id may repeat between a note and a node, because they are different collections. The one break is forward: a pipeline carrying notes is refused by a binary older than this amendment, which is the accepted cost before first release. |

## Freeze Evidence

- A complete v1 golden fixture decodes, converts to the runtime model, encodes,
  and decodes again without semantic drift.
- Negative tests cover unknown fields, release metadata, duplicate graph
  identities, invalid pins, missing versions, and future versions.
- Tests prove an empty draft can be saved but cannot be activated.
- Tests prove omitted node config becomes an empty object.
- Existing router regression tests prove malformed dynamic output edges fail at
  save and activation rather than first execution.
- Interruption tests prove failed catalog writes restore the previous draft.
- Interruption tests prove failed activation commits retain the previous active
  snapshot and remove the candidate.
- Replacement tests prove only the newest committed snapshot remains after
  successful activation.
