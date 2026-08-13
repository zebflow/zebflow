# Zebflow Examples

Examples teach users and protect platform behavior. An example must run, not
only look correct in a document.

## Planned Groups

```text
examples/
├── pipelines/         small pipeline patterns
├── web/               pages and components
├── data/              database and file work
├── maps/              spatial processing and map pages
├── nodes/             composite and WASM packages
└── projects/          complete installable projects
```

## Required Content

Each example needs:

- a short purpose
- required credentials and services
- source files
- setup steps
- one test input
- expected output
- an automated verification command
- cleanup steps when it creates data

Examples must not contain real secrets, private URLs, personal data, or large
production datasets. A project example intended for Hub must also pass package
safety review from a clean project.

The first complete examples should cover a JSON API, a database backed page, a
file upload, a spatial map, a composite node, a WASM node, and a full project
bundle with schema and initial data.
