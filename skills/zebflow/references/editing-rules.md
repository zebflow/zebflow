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
- Put usage docs under `docs/usage/`.
- Put extension/runtime contracts under `docs/developer/`.
- Put server and operational material under `docs/operations/`.
- Keep deeper historical material linked until it is deliberately cleaned up.
