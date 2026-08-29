# Zebflow Contracts

Contracts state what Zebflow promises. Users and platform developers can both
rely on them.

- [Platform contract](./platform.md)
- [Project contract](./project.md)
- [Instance directory](./instance-directory.md)
- [Format contract](./formats.md)
- [Distribution contract](./distribution.md)
- [Offices contract](./offices.md)
- [Confinement contract](./confinement.md)
- [Interface contract](./interface.md)
- [Registered contract kinds](./kinds/README.md)
- [Contract stability matrix](./stability-matrix.md)
- [Versioning contract](./versioning.md)

Shared rules live directly in this folder. The exact schema, ownership, and
lifecycle of each registered kind live under `kinds/<kind>/`.

## Words Used Here

- **Must** means the rule is required.
- **Must not** means the action is forbidden.
- **Should** means follow the rule unless there is a clear written reason not to.
- **May** means the action is optional.

## Order of Authority

1. Contracts define stable meaning and ownership.
2. Machine readable definitions define exact fields and validation.
3. Generated reference exposes those definitions.
4. Usage and developer guides explain them.
5. Examples show tested use.

Source code implements the contract. It does not silently change the contract.

## One Source of Truth

Do not keep separate hand written copies of one schema. Node forms, DSL help,
MCP help, validation, and reference should read the same node definition. The
same rule applies to packages, APIs, errors, and stored formats.

A public feature is not complete until its owner, definition, validation,
reference, tests, and version effect are clear.
