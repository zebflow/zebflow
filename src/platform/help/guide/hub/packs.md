# Packages

Packages are reusable project materials.

Examples:

- pipelines
- templates
- folder bundles
- project starter structures

## Mental model

A package is a versioned, typed, sharable project artifact.
When you add it, Zebflow copies or clones it into your project workspace.
After that, the added files are ordinary project source; Hub does not manage or
track them as an installed dependency.

The two exceptions are `node_bundle` and `rwe_library`: a node bundle
materializes into `data/hub/nodes/` and is recorded in `zeb.lock`; an RWE
library is recorded in `zeb.lock` and `zebflow.yaml` (`rwe.libraries`). Those
two really are tracked dependencies, not copied source.

That is why the product language should be:

- Add
- Clone to project
- Copy into project

and not system-level plugin installation language.

`Pack` was older shorthand. New UI and docs should say `package`.
