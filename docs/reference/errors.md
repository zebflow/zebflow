# Error Reference

Zebflow errors use a stable code and a human readable message.

```json
{
  "ok": false,
  "error": {
    "code": "FW_FILE_REF_READ",
    "message": "value is not a FileRef with a ref"
  }
}
```

## Code Families

- `FW_NODE_` covers pipeline node execution.
- `FW_FILE_REF_` covers FileRef validation and reading.
- `FW_EDGE_` covers pipeline edge validation.
- `HUB_` covers Hub identity, access, packages, and storage.
- `NODE_` covers installed node package validation.
- `PLATFORM_` covers platform services and storage.
- `KV_` covers key value state operations.
- `MS_` covers map publish and style validation.
- `RWE_` covers web compile and render behavior.

An error code should describe the failing boundary, not a changing internal
function name. Messages should say what was invalid and how to correct it. Do
not include credentials, tokens, raw secrets, or full large payloads.

The complete error list is not centralized yet. Until it is generated, search
the constructors in `src/platform/error.rs`, `src/pipeline/model.rs`, and the
owning module.
