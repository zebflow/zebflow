# Automaton Milestone

Independent module (like RWE/language): `src/automaton`, no platform/framework deps. Zebtune = runtime instance.

**Paradigm:** Zeroclaw-style — allowlisted only, super secure.

---

## Vision: Wild Deployment

**Zebtune** = standalone, cost-effective agent optimized for generous quota models (Minimax, GLM, Codex, DeepSeek).

### Core Goals
- Strategic planning with adaptive replanning
- Extensive tool system (file, shell, web, git, code, data)
- Clever memory & history (cache, patterns, persistence)
- Cost optimization (model routing, batching, context reuse)
- Wild deployment (binary, Docker, REST API)

### Intelligence Flow
```
Basic: plan → execute → done
Target: decompose goal → plan → execute → validate → replan if needed → achieve goal
```

---

## Folder Structure

```
src/automaton/
  interface.rs, model.rs, registry.rs, contract.rs, flow.rs, repl.rs, tools.rs
  engines/         noop.rs, loop_.rs, strategic.rs
  actions/         traits.rs, registry.rs, filesystem.rs, shell.rs, web.rs, git.rs, code.rs, data.rs
  security/        policy.rs, validator.rs, sandbox.rs
  memory/          cache.rs, history.rs, patterns.rs, persistence.rs
  planning/        decomposer.rs, validator.rs, replanner.rs, chaining.rs
  config/          security.rs, deployment.rs, user_prefs.rs
  optimization/    router.rs, batching.rs, context.rs
  trace.rs
```

---

## Milestones

**M1 — Loop + Budget** (1w)
- `run()` API, enforce step_budget

**M2 — Actions + Security** (1w)
- Action trait, ActionRegistry, allowlist validation

**M3 — Trace** (1w)
- Structured trace, run reports

**M4 — Agent Engine** (1w)
- LLM loop with tool calls

**M5 — Liberation** (1w)
- Extract standalone crate, clean API

**M6 — Strategic Planning** (2w)
- Goal decomposition, validation, replanning, context chaining

**M7 — Tool System** (2w)
- File ops, shell (allowlist), web (domains), git, code analysis, data

**M8 — Memory** (2w)
- Cache, history, patterns, persistence

**M9 — Config** (1w)
- Security, deployment, user prefs (YAML/TOML)

**M10 — Optimization** (1w)
- Model routing (cheap/smart), batching, context management

---

## Implementation Phases

**Phase 1** (Weeks 1-2): M1, M2, M3 — Foundation
**Phase 2** (Weeks 3-4): M7 — Tool Ecosystem
**Phase 3** (Weeks 5-7): M6 — Strategic Intelligence
**Phase 4** (Weeks 8-9): M8 — Memory & History
**Phase 5** (Weeks 10-11): M9, M10 — Config & Optimization
**Phase 6** (Week 12): M5 — Liberation

---

## Security Model

- Tool allowlist (by name)
- Shell commands allowlist + blocklist (no `rm -rf`, `dd`, etc.)
- Web domain allowlist
- Path boundaries (allowed/blocked paths)
- Rate limits (tools/min, shell/min, web/min)
- Step budget (hard cap)

---

## Tool Categories (M7)

**File:** read, write, list, find, info
**Shell:** `shell(cmd, args)` — only allowlisted: git, curl, grep, find, cat, ls, pwd
**Web:** search, fetch, parse (domain allowlist)
**Git:** status, log, diff, blame
**Code:** analyze, test, lint
**Data:** json_query, csv_parse, sql (read-only)

---

## Memory System (M8)

**Short-term:** Tool cache (60s TTL), conversation history
**Medium-term:** Recent objectives, learned patterns
**Long-term:** User prefs, workflows (persistent JSON/SQLite)

---

## Cost Optimization (M10)

- **Model routing:** cheap for planning, smart for complexity
- **Batching:** group tools in single turn
- **Context:** keep full history (100k tokens), compress only when needed
- **Cache:** aggressive tool result reuse

Target: 60%+ cost reduction vs naive approach

---

## Next Sprint (Weeks 1-2)

1. M1: `run()` API, step_budget enforcement
2. M2: Action trait, registry, policy
3. M2: Loop engine with allowlist
4. M3: Structured trace
5. Tests: allowlist validation, budget exhaustion

---

This keeps automaton **independent, secure, intelligent, cheap, deployable in the wild**.
