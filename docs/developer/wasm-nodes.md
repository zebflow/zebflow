# WASM Nodes

WASM nodes are portable compiled nodes.

Use them for reusable high-performance logic that should install without rebuilding the Zebflow server.

WASM nodes must follow the same installed node definition rules as native and composite nodes. The runtime implementation is different, but the user-facing contract should be consistent.

Large inputs and outputs should use files or references when practical. Passing huge JSON arrays through node payloads is usually the wrong shape for high-throughput work.
