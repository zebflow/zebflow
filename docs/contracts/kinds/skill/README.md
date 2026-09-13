# Skill

Status: **Candidate** — 2026-09-13.

A skill is the procedure for one kind of task, in the Agent Skills format
(`SKILL.md` + optional `references/`, `scripts/`). The help tree says how the
platform is shaped; a skill says when to do what, in what order, and what
proves it worked. An agent loads one when a task matches its description.

## Identity

| | |
| --- | --- |
| Format | `SKILL.md` with YAML frontmatter `name`, `description`, `license` (required); `metadata.*` free; body Markdown |
| Name | `a-z0-9-`, 1–64 chars, equal to the folder name |
| Description | 1–1024 chars; says *when* to use the skill — it is the trigger, and the only part every session reads |
| Body | ≤ 500 lines; detail goes in `references/`, recipes in `scripts/` (text; never executed by the platform) |
| License | SPDX id (`MIT`, `Apache-2.0`, …) or `proprietary`. Blessed skills are MIT. A public hub accepts open licences only |
| Core | `blessed/skills/zebflow-*/`, embedded (`PLATFORM_SKILL_ASSETS`, `build.rs`); the MCP's own — every project lists them, none can opt out, **never a hub item** |
| Extra | `blessed/skill-extras/<name>/`, embedded (`PLATFORM_SKILL_EXTRA_ASSETS`); reach a project only as a shelf package (`zebflow.skill-<name>`, `src/platform/blessed.rs` `blessed_skill_packages`); not listed until added |
| Project | `<source root>/skills/<name>/`; written by the project or added from the hub; shadows a core skill of the same name — clone to own |
| Hub | `asset_kind: skill`, publish source `skill_folder` (`skills/<name>`); Add lands at `skills/<name>/` under the target's source root whatever folder was requested (`src/platform/services/hub.rs` `preview_skill`, `validate_skill_entries`, `HubInstallPlacement`) |
| MCP | tier 1: `start_here` and `skill_list` (name + description); tier 2: `skill_read name=`; tier 3: `skill_read name= path=`; each skill also a prompt (`prompts/list`, `prompts/get`) — `src/platform/mcp/handler.rs` |
| Code | `src/platform/skills/mod.rs` — parsing, listing, shadowing, path containment |

## Rules

1. Tier 1 is cheap or it is useless: a listing entry is one line, and a body
   is read only when the task matches. Nothing puts every body in a prompt.
2. A skill points into `help(topic=…)` for facts; it does not duplicate them.
   When the two disagree, the help (generated or guarded) wins and the skill
   is wrong.
3. Every DSL fence in a skill builds and every MCP tool it names exists
   (`tests/framework/help_matches_implementation.rs` scans `blessed/skills/`
   and `blessed/skill-extras/`).
4. A skill carries no credential, token, URL of a private instance, or
   instruction to run a shell command against the host.
5. Publish and install refuse the same malformed skill by the same check
   (`HUB_SKILL_INVALID`); a skill handed over as a file is verified by the
   receiver again.
6. Derived work names its origin in `metadata.derived_from` and keeps that
   origin's licence terms.

## Evidence

- `src/platform/skills/mod.rs` tests: frontmatter, names, path containment,
  shadowing, every blessed skill well-formed.
- `src/platform/blessed.rs` test `the_shelf_carries_the_blessed_libraries_and_skills`:
  no `zebflow.skill-zebflow-*` package exists; the extras do.
- `src/platform/services/hub.rs` tests: `a_skill_package_is_one_named_folder_with_a_complete_skill_md`,
  `a_skill_installs_at_skills_name_under_the_source_root_regardless_of_target_folder`.
- Live: `skill_list`, `skill_read`, `prompts/list` over MCP against a running
  instance; `start_here` ends with the listing.

## Not decided

- Whether the Studio shows skills in the Hub UI as their own tab (the kind is
  installable today through the API and the Add flow).
- Versioning of a project's cloned copy against the blessed original (a clone
  is source; nothing diffs it).
