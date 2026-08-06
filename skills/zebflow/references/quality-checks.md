# Quality Checks

Before reporting completion:

- confirm changed files
- verify links or routes touched by the work
- run focused tests when code changed
- avoid expensive broad tests for docs-only work
- mention any check that could not be run

For UI work:

- run a local instance when practical
- inspect with a browser
- check desktop and narrow widths for layout changes
- confirm buttons trigger network requests or state changes when expected

For runtime work:

- test the node, route, command, or API path that changed
- check error behavior, not only success behavior
- watch for large payload duplication when testing data-heavy paths

For docs:

- check that linked files exist
- search for stale names after renames
- keep main README short
