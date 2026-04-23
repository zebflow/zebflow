# What Zebflow Is

Zebflow is a project-based runtime that combines:

- pipeline execution
- TSX page rendering
- embedded data capabilities
- agent tooling
- operator surfaces

In a normal web stack, you would split these into separate systems:

- backend APIs
- frontend app
- job runner
- workflow automation
- internal tools

In Zebflow, those pieces are composed inside one project.

## Core idea

A request, schedule, or function trigger enters a pipeline.
That pipeline can:

- query data
- transform payloads
- call external services
- render HTML from a TSX template
- respond as JSON / text / page output

This is why Zebflow projects feel like application systems, not only automations.
