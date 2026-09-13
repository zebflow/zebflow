#!/usr/bin/env bash
# Generate the public skills repository (github.com/zebflow/skills) from
# blessed/skills/. The output is a complete repo working tree: skills, README,
# LICENSE, the Claude Code plugin marketplace manifests, and a context7
# config. Nothing in the output is edited by hand — change blessed/skills/
# and run this again.
#
#   ./scripts/publish-skills.sh                 → .ignored/skills-public/
#   ./scripts/publish-skills.sh /path/to/clone  → into an existing clone; then git diff / commit there
#
# The same SKILL.md files ship inside the binary (PLATFORM_SKILL_ASSETS) and
# are what `skill_list` / `skill_read` serve over MCP; the repo exists so the
# agents that do not sit behind the MCP — Claude Code, Codex, Cursor, Copilot,
# Gemini CLI via `npx skills add zebflow/skills` or the plugin marketplace —
# read the identical text.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/blessed/skills"
OUT="${1:-$ROOT/.ignored/skills-public}"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"

if [ ! -d "$SRC" ]; then
  echo "no $SRC" >&2
  exit 1
fi

mkdir -p "$OUT/skills" "$OUT/.claude-plugin"
# Replace the skills wholesale so a removed blessed skill disappears here too.
find "$OUT/skills" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} +
cp -R "$SRC"/. "$OUT/skills/"

names=()
while IFS= read -r dir; do
  names+=("$(basename "$dir")")
done < <(find "$SRC" -mindepth 1 -maxdepth 1 -type d | sort)

# ── README ───────────────────────────────────────────────────────────────────
{
  cat <<'EOF'
# Zebflow skills

The procedures an agent follows when it builds a Zebflow project: when to do
what, in what order, and what proves it worked. Each folder is one skill in
the [Agent Skills](https://agentskills.io) format — `SKILL.md` with `name`,
`description` (the trigger) and `license`.

These are the **blessed** skills. Every Zebflow project already serves them to
its agent over MCP (`skill_list`, `skill_read`, and as prompts), so an agent
connected to a project needs nothing from this repository. This repository is
for the agents that read skills from a folder instead — Claude Code, Codex,
Cursor, Copilot, Gemini CLI — and for anyone who wants to fork one.

## Install

```bash
npx skills add zebflow/skills                 # all of them, into the agent folders you use
npx skills add zebflow/skills --skill zebflow-basic
```

Claude Code plugin marketplace:

```
/plugin marketplace add zebflow/skills
/plugin install zebflow@zebflow-skills
```

## Skills

EOF
  for name in "${names[@]}"; do
    desc="$(awk 'BEGIN{f=0} /^---/{f++; next} f==1 && /^description:/{sub(/^description:[ ]*/,""); gsub(/^"|"$/,""); print; exit}' "$SRC/$name/SKILL.md")"
    echo "- [\`$name\`](./skills/$name/SKILL.md) — $desc"
  done
  cat <<EOF

## How they relate to the platform

A skill points into the platform's help topics (\`help(topic="…")\` over MCP)
for the facts — node flags, routes, hooks — and never duplicates them. The
help is generated or guarded against the code; when a skill and the help
disagree, the help wins and the skill has a bug.

A project can add its own skills at \`skills/<name>/SKILL.md\` in its
repository, or clone one from the Zebflow Hub; a project skill with the same
name as a blessed one replaces it for that project.

Generated from \`blessed/skills/\` in the Zebflow repository at version
${VERSION} by \`scripts/publish-skills.sh\`. Changes go there.

## License

MIT — see [LICENSE](./LICENSE).
EOF
} > "$OUT/README.md"

# ── LICENSE ──────────────────────────────────────────────────────────────────
cp "$ROOT/LICENSE" "$OUT/LICENSE"

# ── Claude Code plugin + marketplace ─────────────────────────────────────────
skills_json="$(printf '"./skills/%s",' "${names[@]}")"
skills_json="[${skills_json%,}]"
cat > "$OUT/.claude-plugin/plugin.json" <<EOF
{
  "name": "zebflow",
  "description": "Skills for building Zebflow projects: pipelines, pages on the RWE engine, the zeb/ui component set, data, auth, files and the editor, verification, the Hub.",
  "version": "${VERSION}",
  "author": { "name": "Zebflow" },
  "license": "MIT",
  "keywords": ["zebflow", "pipelines", "mcp", "tsx", "zeb-ui", "agent-skills"],
  "skills": ${skills_json}
}
EOF
cat > "$OUT/.claude-plugin/marketplace.json" <<EOF
{
  "name": "zebflow-skills",
  "owner": { "name": "Zebflow" },
  "metadata": {
    "description": "The blessed skills for building Zebflow projects",
    "version": "${VERSION}"
  },
  "plugins": [
    {
      "name": "zebflow",
      "source": "./",
      "description": "Skills for building Zebflow projects: pipelines, pages, zeb/ui, data, auth, files, verification, the Hub.",
      "version": "${VERSION}",
      "author": { "name": "Zebflow" },
      "keywords": ["zebflow", "pipelines", "mcp", "agent-skills"],
      "category": "development",
      "strict": false
    }
  ]
}
EOF

# ── context7 ─────────────────────────────────────────────────────────────────
cat > "$OUT/context7.json" <<'EOF'
{
  "$schema": "https://context7.com/schema/context7.json",
  "projectTitle": "Zebflow skills",
  "description": "Procedures for agents building Zebflow projects: pipelines, TSX pages, zeb/ui, data, auth, files, verification, the Hub.",
  "folders": ["skills"],
  "rules": [
    "A Zebflow project is changed only through its MCP tools or API; never write into its data directory.",
    "Webhook data is under input.body; path params under input.params; a draft pipeline serves nothing until pipeline_activate.",
    "Every TSX file imports what it uses from zeb/react, zeb/ui/<name> or @/…; colours are theme roles, never palette classes.",
    "A 200 response can carry '<!-- RWE component error -->'; read the body and open the page before claiming it works."
  ]
}
EOF

echo "wrote $OUT"
echo "skills: ${names[*]}"
echo
echo "Next: cd $OUT && git init (first time) && git remote add origin git@github.com:zebflow/skills.git && git add -A && git commit && git push"
