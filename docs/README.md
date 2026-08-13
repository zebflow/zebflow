# Zebflow Documentation

This directory contains the main long term knowledge for Zebflow users and
platform developers.

The previous documentation set is preserved outside the active knowledge tree at
`.ignored/legacy-2026-08-14-docs/`. It is historical material, not an authority
for current behavior.

## Documentation Scopes

Zebflow has two audiences:

1. **Users** install and use Zebflow. This includes authoring projects,
   pipelines, pages, composite nodes, WASM nodes, and Hub packages.
2. **Platform developers** change Zebflow itself, including its Rust runtime,
   built-in nodes, RWE engine, storage, platform services, and release process.

Documentation is organized by purpose:

- [Usage](./usage/README.md) teaches users how to build with Zebflow.
- [Developer](./developer/README.md) explains how Zebflow itself works.
- [Contracts](./contracts/README.md) defines rules both audiences may rely on.
- [Reference](./reference/README.md) contains exact schemas, commands, and APIs.
- [Examples](./examples/README.md) contains verified, runnable examples.

## Authority

The contracts define stable meaning and compatibility. Machine-readable source
definitions define exact fields, types, defaults, and validation rules. User
guides and examples may explain those contracts, but may not redefine them.

When documentation, generated help, and implementation disagree, the mismatch
is a defect. It must be resolved at the source of truth rather than documented as
an exception.

## Growth Rule

New documentation is added one verified subject at a time. A document enters
this directory only when it describes current behavior or an approved contract
rule. Conversation notes, experiments, proposals, and obsolete
material belong under `.ignored/`, not here.
