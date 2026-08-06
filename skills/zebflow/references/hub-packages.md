# Hub Packages

Hub distributes inspectable source packages, not backups.

Package families:

- template
- pipeline
- folder
- library
- example
- project bundle
- node bundle

Review before add should show:

- package kind
- publisher
- version
- files to create or replace
- external URLs
- credential requirements
- initialization steps
- database schema or seed steps
- executable scripts
- policy warnings

Naming:

- Use `Hub` for the product surface.
- Use `package` for published shareable items.
- Use `node bundle` for installable node packages.

Do not add silent execution during package review. Any initialization step must be visible before the user confirms.
