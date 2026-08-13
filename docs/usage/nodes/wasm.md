# WASM Nodes

WebAssembly, usually called WASM, is a portable bytecode format. Rust, C, and
other languages can compile to it. Zebflow can load a WASM module without
rebuilding the Zebflow executable.

Use WASM for a focused algorithm that benefits from compiled speed and portable
delivery. Keep network, credentials, storage, and platform policy in Zebflow
unless the host contract explicitly provides them.

## Package Content

A WASM node bundle contains:

- `definition.json` with one or more node definitions
- one or more WASM modules
- optional icons and examples
- checksums recorded by the package format

## Input and Output

Small JSON values can cross the host boundary directly. Large data should use a
FileRef or another stable handle so the host and module do not make several full
copies.

The public node interface must match normal nodes. A user should not need a
different pipeline model because the implementation is WASM.

## Safety and Limits

The host controls module size, input size, output size, memory, time, and allowed
capabilities. A module cannot assume normal operating system access unless the
host grants it.

## Good WASM Design

- Keep repeated inner work inside the module.
- Cross the host boundary a small number of times.
- Return small summaries or stable references for large results.
- Keep the module deterministic when possible.
- declare input, output, errors, and resource needs.
- test the same package on macOS and Linux hosts.

The host implementation is described in the
[node system developer guide](../../developer/node-system.md).
