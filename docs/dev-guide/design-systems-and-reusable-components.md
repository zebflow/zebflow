# Design Systems & Reusable Components

> Status: draft
> Audience: Zebflow frontend developers

---

## Before Reading This Guide

Read **[Template Formal Guide](../user-guide/template-formal-guide.md)** first. It defines the foundational rules:

- Every page is a **Zeb React (RWE) TSX component** — `export default function Page(input)`
- **`@/`** means the template root
- **`.ts` files** are allowed only as pure utility scripts (no DOM access)
- **Tailwind** is the primary styling engine — utility classes in TSX
- **`page` export** handles document-level config (title, stylesheets)

---

## Rule #1 — Zeb React First

Every page must be a **Zeb React (RWE) TSX component**. No exceptions.

```
export default function Page(input) {
  return <div className="...">...</div>;
}
```

**TS files (.ts)** are allowed only as:
- Pure utility helper scripts (pure functions, no DOM access, no side effects)
- API fetch wrappers
- Type definitions

**Forbidden in TS files:**
- `document.querySelector` / `document.getElementById`
- `document.createElement` / `innerHTML` / `innerText`
- Any DOM read/write

---

## Rule #2 — Tailwind Primary

Style in this order of preference:

1. **Utility classes in TSX** — primary method, use first
2. **`styles/main.css`** — CSS variables, fonts, semantic tokens, scrollbar, animations, things Tailwind can't express
3. **`tw-variants`** — dynamic Tailwind classes the compiler can't infer
4. **Inline `style={{}}`** — dynamic geometry only (width, height, transform, coordinates, computed CSS vars)
5. **Raw `<style>` blocks** — local escape hatch only, use sparingly

**`main.css` is NOT for:**
- Layout classes
- Typography
- Colors (use Tailwind or CSS vars)

**`page.links`** for stylesheets — this is correct per template formal guide:

```ts
export const page = {
  links: [{ rel: "stylesheet", href: "/assets/platform/db-suite.css" }],
};
```

---

## Component Library

Path: `@/components/ui/`

### Exists — Use These

| Component | File | Notes |
|-----------|------|-------|
| Button | `button.tsx` | 6 variants, 5 sizes |
| Input | `input.tsx` | |
| Textarea | `textarea.tsx` | |
| Select / SelectOption | `select.tsx` | |
| Checkbox | `checkbox.tsx` | |
| Toggle | `toggle.tsx` | |
| Label | `label.tsx` | |
| Field | `field.tsx` | Label + input wrapper |
| Badge | `badge.tsx` | 4 variants |
| Alert | `alert.tsx` | 4 variants |
| Separator | `separator.tsx` | |
| Kbd | `kbd.tsx` | Keyboard key chip |
| Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter | `card*.tsx` | |
| Tabs, TabsList, TabsTrigger, TabsContent | `tabs.tsx` | |
| StudioTabNav, StudioTabLink | `studio-tab-nav.tsx` | Studio-specific |
| Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter | `dialog*.tsx` | |
| ConfirmDialog | `confirm-dialog.tsx` | |
| DropdownMenu, Trigger, Content, Item, Separator | `dropdown-menu*.tsx` | |
| Sonner | `sonner.tsx` | Toast notifications |
| StudioTable, StudioThead, StudioTh, StudioTd | `studio-data-table.tsx` | |
| TreeView, TreeItem | `tree*.tsx` | |
| HierarchyTree | `hierarchy-tree.tsx` | |
| TemplateFolderTree | `template-folder-tree.tsx` | |
| WebhookRouteTree | `webhook-route-tree.tsx` | |
| CodeEditor | `code-editor.tsx` | |
| ConsolePanel | `console-panel.tsx` | |
| ColorSwatch | `color-swatch.tsx` | |
| Markdown | `markdown.tsx` | |
| HelpTooltip | `help-tooltip.tsx` | |
| ProjectStudioShell | `pages/project-studio/components/shell.tsx` | |
| PlatformSidebar | `platform-sidebar.tsx` | |

### Missing — Need to Build

| Component | Needed For |
|-----------|-----------|
| `SchemaTree` | DB Suite sidebar — tree with expand/collapse, icons, badges, click handlers |
| `QueryEditor` | DB Suite query tab — CodeMirror wrapper with toolbar, history chips, run button |
| `ResultsGrid` | DB Suite results — sortable table with cell selection → inspector panel |
| `InspectorPanel` | DB Suite right panel — key-value, JSON tree, map, image view by data type |
| `MartCard` | Mart tab — saved query card with name, description, last run, run button |
| `HistoryChip` | Query tab — clickable query snippet that re-populates the editor |
| `StatusBadge` | Inline status indicator: ok (green), warn (yellow), error (red), busy (yellow) |

---

## CSS Files

| File | Purpose | Status |
|------|---------|--------|
| `styles/main.css` | CSS vars, fonts, semantic color tokens | Correct — intended use |
| `styles.css` | Project studio layout, `.project-*`, `.pipeline-*`, `.db-suite-*` classes | Needs migration to Tailwind |
| `styles/db-suite.css` | DB Suite layouts (`.db-suite-*`) | Needs migration to Tailwind |
| `styles/db-connections.css` | DB connection list items (15 lines) | Trivial, low priority |

---

## Page Inventory

| Page | RWE | Behavior File | DOM Issues | Notes |
|------|-----|---------------|-----------|-------|
| `login` | NO | — | — | Pure HTML form, server-rendered — acceptable |
| `home` | ✅ | — | — | Full RWE |
| `hub` | ✅ | — | — | Full RWE |
| `project-studio/credentials` | ✅ | ❌ behavior file | ❌ DOM in .ts | Page is RWE, but behavior file does DOM form creation |
| `project-studio/connections` | ✅ | ❌ behavior file | ❌ DOM in .ts | Page is RWE + StudioTable, but behavior file manages dialog |
| `project-studio/dashboard` | PARTIAL | ❌ behavior file | ❌ innerHTML in .ts | RWE shell, behavior file populates metrics via innerHTML |
| `project-studio/files` | PARTIAL | ❌ behavior file | ❌ DOM in .ts | RWE shell, behavior file handles upload/mkdir/delete |
| `project-studio/pipelines` | PARTIAL | ❌ behavior files (x2) | ❌ DOM in .ts | RWE shell, behavior files handle create/delete |
| `project-studio/infrastructure` | ✅ | — | — | Full RWE, sub-components handle state |
| `project-studio/hub` | ✅ | — | — | Full RWE |
| `project-studio/settings` | ✅ | — | — | Full RWE |
| `project-studio/connections/db/sekejap` | ✅ | — | — | Full RWE |
| `project-studio/connections/db/postgresql` | ✅ | — | — | Full RWE |
| `project-studio/connections/db/mapserver` | ✅ | — | — | Full RWE |
| `project-studio/connections/db/connection` | ✅ | ❌ behavior file | ❌ innerHTML in .ts | RWE table structure, behavior file does schema tree + results via innerHTML |
| `project-studio/pipelines/registry` | ✅ | — | — | Thin wrapper → UnifiedRegistryEditor |
| `project-studio/settings/clone/ui/preview` | ✅ | — | — | Full RWE |
| `dev/design-system` | ✅ | ✅ (copy only) | — | Full RWE, behavior file is acceptable (copy-to-clipboard only) |

---

## All Inconsistencies Found

### Behavior Files (`.ts`) Doing DOM Manipulation

| File | Problems |
|------|---------|
| `db-suite-behavior.ts` | `innerHTML` for schema tree and table rendering — rewrite as RWE |
| `credentials-behavior.ts` | `document.createElement`, `innerHTML` for form and list — rewrite as RWE |
| `connections-behavior.ts` | DOM dialog management via `showModal()` — rewrite as RWE |
| `dashboard-behavior.ts` | `innerHTML` string building for metrics — rewrite as RWE |
| `files-behavior.ts` | DOM upload/mkdir/delete — rewrite as RWE |
| `pipelines-behavior.ts` | DOM create/delete/commit dialogs — rewrite as RWE |
| `install-catalog-behavior.ts` | `innerHTML` + inline `style=""` attributes (not Tailwind) — rewrite as RWE |
| `design-system-behavior.ts` | Copy-to-clipboard only — acceptable |

### TSX Files Doing DOM Manipulation (Wrong — Should Be `.ts`)

| File | Problems |
|------|---------|
| `studio-shell-behavior.tsx` | Uses `document.querySelector`, `document.body.appendChild`, `innerHTML` — 839 lines. Should be `studio-shell-behavior.ts` |
| `project-console.tsx` | TypeScript class with `document.querySelector` in `waitForSelector` — should be `.ts` |
| `console-panel.tsx` | Uses `document.querySelector` in `useEffect` for focus — minor, could use ref instead |

### CodeMirror Integration (Acceptable Exception)

| File | Notes |
|------|-------|
| `node-field-code-editor.tsx` | Uses `containerRef.current.innerHTML = ""` for CodeMirror mount — editor integrations require direct DOM |
| `web-render-dialog.tsx` | Same pattern — editor integration requires DOM |

### Inline Styles Not Tailwind

| File | Line | Issue |
|------|------|-------|
| `install-catalog-behavior.ts` | 63, 71, 78, 81-87 | `style="padding:16px;color:var(--color-ui-text-muted);font-size:12px;"` — should use Tailwind classes |

### Naming Inconsistency

| File | Issue |
|------|-------|
| `studio-shell-behavior.tsx` | Has "behavior" in name but is `.tsx` — should be `.ts` |
| `project-console.tsx` | Is a TypeScript class, not a React component — should be `.ts` |

---

## CSS Files Usage

Pages using `page.links` for stylesheets (correct per template formal guide):

| Page | Stylesheets |
|------|------------|
| `connections/db/sekejap` | db-suite.css, devicons.css |
| `connections/db/postgresql` | db-suite.css, devicons.css |
| `connections/db/mapserver` | db-suite.css |
| `connections/db/connection` | db-suite.css, devicons.css |
| `connections` | db-connections.css, devicons.css |
| `pipelines` | devicons.css |
| `pipelines/registry` | devicons.css |
| `hub` | db-suite.css |

All other pages rely on `styles.css` (project studio global) or no extra stylesheet.

---

## Summary — Migration Work

| # | Task | Severity | Effort |
|---|------|---------|--------|
| 1 | Rewrite `db-suite-behavior.ts` → RWE components (`SchemaTree`, `ResultsGrid`, `InspectorPanel`) | High | High |
| 2 | Rewrite `credentials-behavior.ts` → RWE Dialog components | High | Medium |
| 3 | Rewrite `connections-behavior.ts` → RWE Dialog components | High | Medium |
| 4 | Rewrite `dashboard-behavior.ts` → RWE `DashboardMetrics` component | Medium | Medium |
| 5 | Rewrite `files-behavior.ts` → RWE components | Medium | Medium |
| 6 | Rewrite `pipelines-behavior.ts` → RWE Dialog components | Medium | Medium |
| 7 | Rewrite `install-catalog-behavior.ts` → RWE components + Tailwind | Low | Low |
| 8 | Rename `studio-shell-behavior.tsx` → `.ts` | Low | Low |
| 9 | Rename `project-console.tsx` → `.ts` | Low | Low |
| 10 | Build missing components: `SchemaTree`, `QueryEditor`, `ResultsGrid`, `InspectorPanel`, `MartCard`, `HistoryChip`, `StatusBadge` | High | High |
| 11 | Migrate `db-suite.css` → Tailwind | Low | Low (deferred) |
