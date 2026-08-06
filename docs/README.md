# Zebflow Documentation

Zebflow documentation is split by audience.

- [Usage](./usage/README.md) is for people building apps, APIs, maps, workflows, databases, and AI tools with Zebflow.
- [Developer](./developer/README.md) is for people extending Zebflow itself or creating reusable nodes and packages.
- [Operations](./operations/README.md) is for people running Zebflow on a machine, server, or cluster.
- [Examples](./examples/README.md) is for runnable examples that can also become Hub packages.

## Existing Reference Docs

Some detailed material still lives in the older folders while the docs are being reshaped:

- [User Guide](./user-guide/README.md)
- [Developer Guide](./dev-guide/README.md)
- [Project Contract](./dev-guide/project-contract.md)
- [Office Federation Contract](./dev-guide/office-federation-contract.md)
- [Architecture](./dev-guide/architecture.md)
- [Hub Formal Guide](./user-guide/hub-formal-guide.md)
- [Storage Formal Guide](./user-guide/storage-formal-guide.md)

## Writing Rule

Write Zebflow docs for global users first: clear words, short examples, and practical steps. Put deep architecture details in the developer guide, not in the first page a new user reads.

## Coverage Map

Use [Coverage Map](./coverage-map.md) to check where each major Zebflow source area is documented. If a new top-level module, project surface, route family, node family, package type, or runtime subsystem is added, update that map in the same change.
