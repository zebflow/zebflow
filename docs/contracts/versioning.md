# Versioning Contract

The runtime version and each stored format version are separate. A Zebflow
release can read several format versions. One project can contain several kinds
of versioned files.

## Patch Release

A patch release may fix bugs, security, speed, memory use, validation, and error
messages. It may reject input that was already invalid.

A patch release must not change the meaning of valid source, stored data, public
paths, schemas, or documented behavior.

## Minor Release

A minor release may add optional fields, node kinds, APIs, and new capabilities.
Existing valid projects must keep working without a required rewrite.

A minor release must not silently remove, rename, or change the meaning of a
stable field, node, route, path, or behavior.

## Major Release

A major release is required for an incompatible stable change. It must provide:

- a list of affected formats and behavior
- a project check before update
- a migration or a clear refusal
- backup and recovery steps
- safe failure without a half changed project

## Safe Update

Before changing durable state, Zebflow must:

1. list the project's formats and dependencies
2. compare them with the target runtime
3. report unknown or incompatible items
4. make the required backup or checkpoint
5. run supported migrations as one safe operation
6. verify the result before activation
7. keep the previous state when verification fails

## Release Completion

A release that changes a stable surface is incomplete until it updates the
format record, contract, generated reference, migration rules, and compatibility
tests.
