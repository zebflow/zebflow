# Package Reference

Zebflow packages reusable source. Package content is reviewed before it changes
a project.

## Hub Package Kinds

- `pipeline_bundle`
- `template_bundle`
- `folder_bundle`
- `project_bundle`
- `node_bundle`

Unknown kinds are rejected.

## Common Package Identity

A Hub package has a package ID, publisher ID, kind, title, description,
visibility, tags, and versions that do not change after publishing. Each version
records its source, stored package path, checksum, and manifest.

## Node Bundle

Every node bundle uses one `definition.json`, whether it contains one node or
many.

```text
package/
├── definition.json
├── icon.svg
├── functions/
│   └── function.zf.json
└── wasm/
    └── module.wasm
```

Only files needed by the package are required. Composite bundles use function
pipelines. WASM bundles use one or more modules. Both may share credential type
definitions.

`definition.json` is a `NodeBundle` document using `zebflow.com/v1`. Its
`metadata.name` and `metadata.version` match `spec.package` and `spec.version`.
A node entry uses the normal node definition fields. The package loader turns
every entry into an installed `NodeDefinition` document.

## Safety Review

Review reports files, node kinds, credentials, URLs, database effects, file
effects, public endpoints, schedules, large files, initial data, warnings, and a
risk level.

Related source:

- `src/platform/model.rs`
- `src/platform/services/hub.rs`
- `src/platform/services/node_registry.rs`
- `src/platform/policy/package.rs`
