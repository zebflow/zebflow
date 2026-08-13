# Projects

A project is the main unit of work in Zebflow. Pages, pipelines, data, files,
credentials, and settings belong to a project.

## Project Areas

```text
project/
├── repo/     source that can be tracked with Git
├── data/     databases and runtime owned data
└── files/    uploads and generated file objects
```

The source area contains:

```text
repo/
├── zebflow.json
├── zeb.lock
├── pipelines/
└── docs/
```

`repo/pipelines/` is the live source tree. It can contain pipeline files, TSX
pages, components, scripts, styles, and assets. `repo/docs/` contains notes that
belong to the project. `zebflow.json` stores project settings that are safe to
track. `zeb.lock` records selected libraries and package state.

## Normal Work Loop

1. Create or open a project from Home.
2. Add a page, pipeline, connection, credential, or file in Project Studio.
3. Test the work before activation.
4. Activate pipelines that should receive traffic.
5. Review Git changes.
6. Commit one clear unit of work.

## Source and Data Are Different

Source is meant to be read, reviewed, copied, and versioned. Data may be large,
private, or changed by live traffic. Do not put credentials, production data,
runtime logs, or temporary files into the source tree.

## Locks

Project owners can lock selected source from tool based editing. A lock protects
the editing path. It does not replace project access control or Git history.

## Moving a Project

Use project transfer or a project bundle when you want another Zebflow instance
to recreate the project. Use Git for source history. Use backup and restore for
full operational recovery. Hub packages are for reusable source, not complete
production backups.

Stable project rules are defined in the
[project contract](../contracts/project.md).
