# UX

Status: **Proposed** — 2026-09-13. What every Studio screen owes the person
using it, so nothing has to be learned twice. Clean Code, for interfaces.

## 0. The rule

**No screen asks the user to hold the platform in their head.** A thing on
the screen is understood from its name; when a name is not enough, the
screen shows rather than tells; help is the last resort, never the first.

## 1. The clarity ladder

For every element, use the **lowest** rung that removes the question. Going
higher when a lower rung would do is a defect, the same as a comment that
explains a badly named function.

| Rung | What it is | Use when | Example |
|---|---|---|---|
| 1 Name | the label, title, button text | always — most things end here | `Production hosts`, not `Domains config` |
| 2 Inline line | one short sentence under the label, or a placeholder that is a real example | the name says *what*, the reader also needs *so what* or the shape | `northside.example — DNS must point at this instance`; placeholder `2026-09-14T09:00` |
| 3 Showing | an example value, a live stat, a preview, a generated artefact, a status chip | the consequence is easier to see than to read | "3 layers published", the rendered page title next to a route, DNS ✓/✗ |
| 4 `(?)` note | a popover of 1–3 sentences on a section, a subsection or a single input, ending in "Read more →" | a *why* or a *what never changes* that would clutter the screen | the hosts section: "the project never stores a host, so it moves as-is" |
| 5 Help topic | the full explanation in the Help panel (top right), with `studio://` links back into the screens and a `video:` row when a tutorial exists | learning how to do something across screens | `platform/addressing` |

Rungs 4 and 5 are one corpus in two granularities: a note is
`help/ui/<screen>/<anchor>.md` with a `topic:` pointing at its topic and
heading; a topic is `help/<area>/<name>.md`, the same text `help(topic=…)`
serves an agent. Notes are for humans and are not indexed for agents.

## 2. Naming

1. **Nouns for things, verbs for actions, the contract's words for both** —
   host, route, surface, pipeline, template, credential, package. A screen
   does not invent a synonym or a cute name.
2. **A title says what the screen decides**, not what it contains:
   `Addressing`, not `URL settings`; `Who can see this project`, not `ACL`.
3. **A button says what happens**: `Add host`, `Copy Nginx config`,
   `Remove host` — never `Submit`, `OK`, `Apply` alone.
4. **A status says the state and the next step**: `DNS → 1.2.3.4 ✗ — add: A northside 203.0.113.7`,
   not `Error`.
5. **Units and formats are in the label or the placeholder**, never
   discovered by a failed save: `Timeout (seconds)`, `Cron (5 fields, UTC)`.

## 3. Show, don't tell

Add an interactive element, a stat, an example or a preview **when it
replaces a paragraph**, and not otherwise:

- a generated artefact instead of instructions to write one (the Nginx block);
- a live check instead of "make sure that…" (DNS status, `Verify` after apply);
- a real example value in a placeholder instead of a format description;
- a count or a preview beside a choice (`Map tiles — 3 layers published`,
  the page title beside a route) instead of a sentence about what it serves;
- a diff or a before/after when a change is destructive.

## 4. Every section, the same shape

```
[Title]                                                     (?) only if rung 4 is earned
  why      — 1–3 sentences: what this decides and what it never changes (may be empty when the title suffices)
  inputs   — the user's words; pickers over free text; examples as placeholders
  output   — the copyable artefact, when there is one
  verify   — the live check, when there is one
```

Addressing (`addressing.md` §6) is the reference implementation. A section
without an output or a verify simply has none — it does not invent one.

## 5. Rules that hold on every screen

1. **Every address is copyable where it is seen** and never composed by hand.
2. **A conflict names the other party**, with a link, never a silent refusal.
3. **Defaults are invisible until asked** — one `Advanced ▾` per screen.
4. **Destructive means confirm with the consequence in words**, not "Are you sure?".
5. **An empty state is an instruction** with the action that fills it.
6. **A generated artefact carries its check** — the command that proves it worked.
7. **The same fact appears once**: not in an inline line *and* a note *and* the topic.
8. **Zeb React and zeb/ui only** for any new or rewritten screen (`CLAUDE.md`
   Absolute UI Rule); one component per file, ≤ 400 lines.

## 6. Not covered

- Visual identity (palette, type, spacing): `web/tailwind` theme roles and
  `/dev/design-system` are the source; this contract is behaviour and words.
- Marketing pages and the public homepage.

## Evidence

- Every `data-help` anchor on a platform template resolves to a note file,
  every note's `topic:` to a topic and heading, every `studio://` link in a
  topic to a route that exists (`tests/framework/help_matches_implementation.rs`, to add).
- Settings sections are built from the section component; a bare `<h2>`
  under `templates/.../settings/**` is refused (template test, to add).
- A button whose text is `Submit`, `OK` or `Apply` alone is refused
  (template lint, to add).
- Addressing tab: rungs 1–3 carry the screen; two `(?)` notes at most; the
  generated configs each end in their check command.
