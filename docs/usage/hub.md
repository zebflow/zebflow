# Hub

Hub shares reusable Zebflow source. It is not a backup service and it does not
take ownership of files after they are added to a project.

## Main Terms

- A Hub service stores and serves packages.
- A publisher owns package names and versions.
- A token gives read, publish, or manage access.
- A Hub source lets a platform or project browse a Hub.
- A grant shares a platform owned source with selected projects.

Owning a Hub service does not automatically give every project access. Access is
explicit.

## Package Kinds

The known package kinds include pipeline, template, folder, project, and node
bundles. The package reference owns the exact names and fields.

## Add Flow

1. Browse a Hub source.
2. Select a package and version.
3. Choose the target folder.
4. Review files, nodes, credentials, URLs, database effects, public routes,
   schedules, large files, and seed data.
5. Resolve path conflicts.
6. Add the package as project source.
7. Review and activate imported pipelines yourself.

Adding a package copies source into the project. You may edit it afterward. Hub
does not track it as a locked installation.

## Publish Flow

1. Select project source.
2. Choose a package kind, slug, title, and version.
3. Add useful tags and optional media.
4. Review private paths, secrets, external effects, and file size.
5. Publish with a scoped token.

Never publish credentials, runtime databases, logs, Git internals, or production
data in a normal Hub package.
