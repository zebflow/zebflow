# NodeDefinition

Status: **Frozen**

`NodeDefinition` describes one node kind in a way that does not depend on its
implementation. Native Rust, composite, and WASM nodes use the same fields in
the node catalog, pipeline editor, DSL help, MCP help, and runtime validation.

It does not describe a node instance in a pipeline. It also does not own bundle
files, release versions, credentials, composite functions, WASM modules, or
installation. Those belong to `NodeBundle`.

## Identity

| Item | Value |
| --- | --- |
| API version | `zebflow.com/v1` |
| Kind | `NodeDefinition` |
| Representation | Strict UTF-8 JSON envelope |
| Owner | Pipeline node system |
| Rust model | `src/pipeline/model.rs` |
| Contract and validator | `src/contracts/kinds/node.rs` |
| Runtime registry | `src/platform/services/node_registry.rs` |
| Golden fixture | `tests/fixtures/contracts/node-definition/v1-complete.json` |

`metadata.name` must equal `spec.kind`. `metadata.version` is forbidden because
a node definition is an interface, not a released package. A `NodeBundle` owns
the package release version.

## Canonical Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "NodeDefinition",
  "metadata": {
    "name": "n.example.echo"
  },
  "spec": {
    "kind": "n.example.echo",
    "title": "Example Echo",
    "description": "Return the incoming payload without changing it.",
    "config_schema": {
      "type": "object",
      "additionalProperties": false
    },
    "input_schema": {
      "type": "object"
    },
    "output_schema": {
      "type": "object"
    },
    "examples": [
      {
        "title": "Echo an object",
        "input": { "message": "hello" },
        "output": { "message": "hello" }
      }
    ],
    "failure_semantics": [
      {
        "code": "FW_NODE_EXAMPLE_INVALID_INPUT",
        "description": "The example input cannot be represented as JSON.",
        "retryable": false
      }
    ],
    "input_pins": ["in"],
    "output_pins": ["out"],
    "script_available": false,
    "script_bridge": null,
    "ai_tool": {
      "registered": false,
      "tool_name": "",
      "tool_description": "",
      "tool_input_schema": null
    },
    "dsl_flags": []
  }
}
```

The complete serialized document is limited to 512 KiB.

## Field Rules

`spec.kind` is a stable lowercase, dot-separated `n.*` identifier. A segment may
also contain an underscore. Renaming it creates a different node kind.

`title` and `description` are required. They are the source for the node picker,
generated help, and agent-readable node documentation.

The three schemas have separate meanings:

| Schema | Meaning |
| --- | --- |
| `config_schema` | Static configuration stored on a pipeline node instance |
| `input_schema` | Payload read from an upstream node |
| `output_schema` | Payload produced for downstream nodes |

Each schema must be a JSON object or `null`. The configuration schema is used
for pipeline registration. Input and output schemas document the payload flow.

`examples` provides concrete config, input, and output values for generated
documentation and tool callers. Every example needs a title. Example config
must be a JSON object or `null`.

`failure_semantics` documents stable machine-readable error codes. Every entry
needs a unique code and a description. It may state whether retry can succeed
and give a retry hint. Runtime errors may include more detail, but must not
change the documented meaning of a code.

Input and output pins are compact unique tokens. A trigger can have no input
pin. A node with graph-defined outputs can have no fixed output pins.

Every user-facing configuration property must be documented by a UI field or a
DSL flag. Field names are unique. Layout entries may only reference declared
fields. DSL flag names are unique. Two flags may target the same configuration
key only when they provide distinct representations, such as field-by-field and
full-schema input.

`script_available` and `script_bridge` must be declared together. A bridge name
uses an `n.*` identifier. That coherence rule is the whole of what the pair
means today: **no runtime dispatches from a script into a node handler**, so
neither field grants a call nor prevents one. A script's `n` object holds pure
time and arithmetic helpers, and the sandbox exposes no op that reaches a node.
Every definition Zebflow ships declares `false` and `null`, and a reader must
not take that as a restriction being enforced. The fields are reserved shape
for a bridge that does not exist; removing them from `spec` is a serialized
field removal, which the compatibility rules below place in a new contract API
version with an explicit converter, not in a v1 patch.

An AI tool marked as registered requires a stable tool name, description, and
object or null input schema. An unregistered AI tool may not carry hidden tool
metadata.

Unknown fields are rejected in the envelope and every typed object.

## Boundaries

### Native Nodes

Native definitions are compiled into the Zebflow binary. Tests validate every
native definition before release.

### Composite And WASM Nodes

An install source always uses one `NodeBundle` in `definition.json`, even when
it provides one node. The loader normalizes every bundle entry into the same
`NodeDefinition` used by native nodes. The source package is not rewritten into
legacy `node.json` files.

Composite and WASM details stay outside this contract:

- composite functions and lifecycle hooks belong to `NodeBundle`
- WASM module paths, exports, and ABI belong to `NodeBundle`
- credentials belong to `NodeBundle`
- icons are bundle artifacts referenced by a node entry
- release identity and integrity belong to `NodeBundle` and `DependencyLock`

Only `zebflow-wasm-json-v1` is accepted for the current WASM host. An unknown
ABI fails validation and execution. Zebflow does not guess another calling
convention.

### API, UI, And Help

The node API derives its catalog item from the normalized definition. Runtime
metadata such as source, tier, icon URL, and credential requirements is added
as a view. It does not change the definition.

Project Studio renders node fields and layout from this catalog. MCP and help
use the same definitions. No separate TypeScript node catalog owns the schema.

### Pipeline Runtime

Pipeline registration checks the instance kind, pins, and configuration against
the active catalog. A normal pipeline hit uses the already validated runtime
plan. It does not parse or validate node definition files on every request.

## Concurrency And Failure

Project node bundles are scanned into a complete temporary registry. Zebflow
publishes that registry through one atomic pointer swap only after every bundle
and node definition passes validation and the dependency lock update succeeds.

An invalid bundle, missing `definition.json`, duplicate kind, official-kind
collision, unreadable directory entry, or symlinked package directory fails the
refresh. The previous valid registry remains active. Zebflow does not silently
skip an invalid project package and publish a partial catalog.

Official embedded bundles fail at startup if they are malformed or collide.
Build tests validate all official native and composite definitions.

Bundle staging, installation rollback, uninstall safety, artifact existence,
and digest verification belong to the `NodeBundle` review. They are not claimed
as guarantees of `NodeDefinition`.

## Security

A definition contains public schema and documentation only. It must not contain
credential values, tokens, passwords, cookies, signed URLs, or private keys.

Credential kinds and configuration keys may be documented by a bundle, but
credential values remain in the credential service. Runtime linkage paths are
normalized package-relative paths and are never fields in `NodeDefinition`.

## Compatibility

Patch releases may improve validation messages, catalog lookup, and registry
performance without changing accepted or emitted fields.

Minor releases may populate existing optional fields more completely. They may
not rename fields, change field meaning, loosen kind identity, change schema
roles, add guessed aliases, or introduce another WASM calling convention under
`zebflow.com/v1`.

Adding or removing a serialized field, changing required values, changing pin
meaning, or changing script and AI capability semantics requires a new contract
API version and an explicit converter. A v1 reader rejects unknown and newer
formats.

There is no normal compatibility reader for legacy `node.json`. Zebflow is
pre-release, and `definition.json` is the one canonical install source.

## Test Evidence

The implementation tests prove:

1. deterministic golden-file round-trip
2. strict identity and version rules
3. unknown-field and size rejection
4. duplicate-pin and unsafe-path rejection
5. exact WASM ABI validation
6. payload schema preservation during bundle normalization
7. complete native and official composite definitions
8. fail-closed atomic registry refresh
9. cross-bundle kind collision rejection
10. per-node icon resolution

UI verification must also confirm that native and composite nodes appear in the
same picker, open through the shared node dialog, and retain fields, pins,
schemas, icons, source, and credential metadata.
