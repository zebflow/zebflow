# ProjectPlacement

Status: pending review

Renamed from `RuntimeBundle` on 2026-08-29: the old name said nothing about
what it governs. Zebflow already calls this idea *placement*
(`src/infra/execution/placement/`, `ProjectRuntimePlacement`,
`placement_generation`).

This contract defines what one Zebflow sends to another so a project runs
there: which office, which project, which release, which pipelines active, and
which secrets the office must already hold. Its review will cover what travels
(the release itself, or a reference to it), destination ownership, staged
cutover, and rollback.
