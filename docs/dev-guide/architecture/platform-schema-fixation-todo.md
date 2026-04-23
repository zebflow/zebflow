# Platform Schema Fixation TODO

Temporary execution checklist for turning the current platform catalog into the locked long-term Zebflow control-plane schema.

Scope:

- multi-user
- multi-project
- multi-office
- runtime management
- marketplace management

Rules:

- no feature expansion in this track
- migrations first
- security first
- forward-only schema evolution

## Foundation

- [x] Lock the target long-term contract in [platform-data-structure.md](/Users/mala0061/Dev/mecha.id/zebflow/docs/dev-guide/architecture/platform-data-structure.md)
- [x] Create this tracked execution checklist
- [x] Add `schema_migrations` backbone to platform SQLite
- [x] Move current implicit schema drift into ordered migrations
- [x] Turn on and rely on real foreign keys after relational refactor lands

## Identity

- [x] Introduce stable internal `user_id`
- [x] Split `users` and `user_local_auth`
- [x] Introduce stable internal `project_id`
- [x] Stop using `(owner, project)` as the long-term internal identity

## Project Access

- [x] Normalize `project_members` as the durable base access layer
- [x] Relegate richer policy structures to secondary/optional layers
- [x] Add stable uniqueness and FK rules for membership and project ownership

## Runtime / Offices

- [x] Introduce explicit `offices`
- [x] Introduce explicit `office_nodes`
- [x] Normalize `project_runtime_placements` onto office/node ids
- [x] Normalize `project_operations` onto project/office ids

## Marketplace

- [x] Introduce explicit `marketplace_authorities`
- [x] Normalize publishers onto authority ids
- [x] Normalize marketplace tokens onto authority id + publisher pk
- [x] Normalize packages onto authority id + publisher pk
- [x] Normalize package versions onto package pk
- [x] Normalize platform marketplace browsing sources onto stable ids

## Constraints / Hardening

- [x] Add FK constraints to core ownership graph
- [x] Add unique constraints for stable slugs and route-safe identifiers
- [x] Replace FK-sensitive `INSERT OR REPLACE` writes with UPSERT updates
- [x] Reduce JSON dependence in control-plane relationships
- [x] Define soft-disable vs delete lifecycle rules
- [x] Define retention rules for operational tables

## Cutover

- [x] Prepare fresh `0.4.x` data line / PVC strategy
- [x] Document no-in-place-upgrade assumption for pre-contract state
- [x] Verify bootstrap on a clean catalog
- [x] Verify migration from current local dev catalog shape
- [x] Verify build against the migration-backed schema
- [x] Verify deployment manifests against the migration-backed schema
