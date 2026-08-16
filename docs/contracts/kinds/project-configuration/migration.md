# ProjectConfiguration Migration

Migration is an explicit operator action. Normal project reads never convert a
legacy file.

## Command

```bash
zebflow project config migrate <owner> <project>
```

The command uses `ZEBFLOW_PLATFORM_DATA_DIR` when set. Otherwise it uses the
normal platform data root.

Run it while the target project is not receiving configuration writes. The
service serializes updates inside one Zebflow process, but an offline CLI cannot
share that in-memory lock with a running server.

## Accepted Legacy Source

The source must be `repo/zebflow.json` with legacy version `1.0` and this root
shape:

```text
version
metadata
configs
distribution
```

The explicit migration path also accepts an intermediate JSON document that
already uses the `zebflow.com/v1` `ProjectConfiguration` envelope.

## Field Mapping

| Legacy field | Canonical field |
| --- | --- |
| `version` | Removed after verifying `1.0` |
| `metadata.title` | `spec.profile.title` |
| `metadata.description` | `spec.profile.description` |
| `configs.rwe` | `spec.rwe` |
| `configs.pipelines` | `spec.pipelines` |
| `configs.runtime` | `spec.runtime` |
| `configs.bootstrap` | `spec.bootstrap` |
| `configs.git` | `spec.git` |
| `configs.assistant` | `spec.assistant` |
| `configs.locks` | `spec.locks` |
| `configs.data` | `spec.data` |
| `configs.files` | `spec.files` |
| `distribution.marketplace` | `spec.distribution.hub` |
| Intermediate `distribution.hub` | `spec.distribution.hub` |
| Project slug from the owning path | `metadata.name` |

Legacy `configs.data` and `configs.pipelines.nodes` must be empty. If both
`distribution.marketplace` and `distribution.hub` exist, migration fails.
Unknown fields, invalid values, and unrecognized versions also fail. Nothing is
silently dropped or clamped.

## Recovery Sequence

1. Lock updates for the target project configuration.
2. Refuse migration when canonical `zebflow.yaml` already exists.
3. Read and fully validate `zebflow.json`.
4. Write the exact original bytes to `zebflow.pre-yaml.json` atomically. Refuse
   to overwrite an existing recovery file with different bytes.
5. Validate and atomically write `zebflow.yaml`.
6. Reopen and validate `zebflow.yaml` and compare its typed content.
7. Remove `zebflow.json` only after every previous step succeeds.

A validation failure leaves the legacy source untouched. If an operating-system
failure occurs after the canonical file is written, the recovery copy remains
available for inspection or restoration.
