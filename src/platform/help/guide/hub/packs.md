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

That is why the product language should be:

- Add
- Clone to project
- Copy into project

and not system-level plugin installation language.

`Pack` was older shorthand. New UI and docs should say `package`.
