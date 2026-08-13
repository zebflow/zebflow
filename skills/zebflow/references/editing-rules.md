# Editing Rules

Before editing:

- Inspect the current files.
- Check related conventions in nearby modules.
- Preserve unrelated dirty work.
- Keep generated or local-only material under `.ignored/` when it should not be committed.

For Rust:

- Prefer existing modules and helpers.
- Add focused tests when changing runtime behavior.
- Use structured parsers and typed data where available.
- Keep comments short and only where they reduce future confusion.

For UI:

- Use Zeb React and Zeb Tailwind.
- Keep project-studio screens compact.
- Fix reusable components when multiple screens share the problem.
- Avoid local one-off behavior when a shared component should own it.

For docs:

- Main README should stay concise and user-facing.
- Treat `docs/contracts/` as the stable public authority.
- Add one verified subject at a time; do not restore archived documentation in bulk.
- Put proposals, experiments, stale material, and conversation notes under `.ignored/`.
- Do not link active docs or skills to archived documentation.
- Update the relevant contract when changing a stable public surface.
