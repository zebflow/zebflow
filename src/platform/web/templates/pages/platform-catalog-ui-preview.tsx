import { cx } from "zeb";
import { useState } from "zeb";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";

export const page = {
  head: { title: "UI Component Catalog · Zebflow" },
  body: { className: "bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)]" },
  navigation: "history",
};

export const app = {};

const CATEGORY_LABELS: Record<string, string> = {
  primitives: "Primitives",
  display:    "Display",
  layout:     "Layout",
  navigation: "Navigation",
  overlay:    "Overlay",
  complex:    "Complex",
};

const CATEGORY_ORDER = ["primitives", "display", "layout", "navigation", "overlay", "complex"];

export default function CatalogUiPreview(input: any) {
  const components: any[] = Array.isArray(input?.components) ? input.components : [];
  const [activeCategory, setActiveCategory] = useState("all");
  const [search, setSearch] = useState("");

  const categories = CATEGORY_ORDER.filter(cat =>
    components.some(c => c.category === cat)
  );

  const filtered = components.filter(c => {
    const matchCat = activeCategory === "all" || c.category === activeCategory;
    const matchSearch = !search || c.name.toLowerCase().includes(search.toLowerCase()) || c.description.toLowerCase().includes(search.toLowerCase());
    return matchCat && matchSearch;
  });

  const grouped = CATEGORY_ORDER.reduce<Record<string, any[]>>((acc, cat) => {
    const items = filtered.filter(c => c.category === cat);
    if (items.length > 0) acc[cat] = items;
    return acc;
  }, {});

  const total = components.length;
  const installed = components.filter(c => c.installed).length;

  return (
    <div className="min-h-screen">
      {/* Header */}
      <div className="border-b border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg-subtle)] px-6 py-5">
        <div className="mx-auto max-w-6xl">
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-xl font-semibold text-[var(--zf-ui-text)]">UI Component Catalog</h1>
              <p className="mt-1 text-sm text-[var(--zf-ui-text-muted)]">
                shadcn-compatible Zeb React components — install into{" "}
                <code className="rounded bg-[var(--zf-ui-bg-muted)] px-1 py-0.5 font-mono text-xs">shared/ui/</code>
              </p>
            </div>
            <div className="flex items-center gap-3 text-sm text-[var(--zf-ui-text-muted)]">
              <span><strong className="text-[var(--zf-ui-text)]">{total}</strong> components</span>
              <span>·</span>
              <span><strong className="text-[var(--zf-ui-text)]">{Object.keys(CATEGORY_LABELS).length}</strong> categories</span>
            </div>
          </div>

          {/* Search + filter */}
          <div className="mt-4 flex flex-wrap items-center gap-3">
            <input
              type="search"
              placeholder="Search components…"
              value={search}
              onChange={(e: any) => setSearch(e.target.value)}
              className="h-8 w-64 rounded-md border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] px-3 text-sm placeholder:text-[var(--zf-ui-text-muted)] focus:outline-none focus:ring-1 focus:ring-[var(--zf-color-brand-blue)]/40"
            />
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setActiveCategory("all")}
                className={cx(
                  "h-7 rounded-full px-3 text-xs font-medium transition-colors",
                  activeCategory === "all"
                    ? "bg-[var(--zf-ui-text)] text-[var(--zf-ui-bg)]"
                    : "border border-[var(--zf-ui-border)] text-[var(--zf-ui-text-soft)] hover:bg-[var(--zf-ui-bg-muted)]"
                )}
              >
                All ({total})
              </button>
              {categories.map(cat => {
                const count = components.filter(c => c.category === cat).length;
                return (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => setActiveCategory(cat)}
                    className={cx(
                      "h-7 rounded-full px-3 text-xs font-medium transition-colors capitalize",
                      activeCategory === cat
                        ? "bg-[var(--zf-ui-text)] text-[var(--zf-ui-bg)]"
                        : "border border-[var(--zf-ui-border)] text-[var(--zf-ui-text-soft)] hover:bg-[var(--zf-ui-bg-muted)]"
                    )}
                  >
                    {CATEGORY_LABELS[cat] ?? cat} ({count})
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="mx-auto max-w-6xl px-6 py-8 space-y-10">
        {Object.keys(grouped).length === 0 ? (
          <div className="text-center py-16 text-[var(--zf-ui-text-muted)]">No components match your filter.</div>
        ) : (
          Object.entries(grouped).map(([cat, items]) => (
            <section key={cat}>
              <h2 className="mb-4 text-sm font-semibold uppercase tracking-widest text-[var(--zf-ui-text-muted)]">
                {CATEGORY_LABELS[cat] ?? cat}
              </h2>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {items.map((comp: any) => (
                  <div
                    key={comp.name}
                    className="group relative rounded-lg border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] p-4 hover:border-[var(--zf-color-brand-blue)]/40 transition-colors"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="text-sm font-semibold text-[var(--zf-ui-text)]">{comp.name}</p>
                        <p className="mt-0.5 text-xs text-[var(--zf-ui-text-muted)]">{comp.description}</p>
                      </div>
                      <span className={cx(
                        "shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium",
                        comp.installed
                          ? "bg-green-500/10 text-green-600"
                          : "bg-[var(--zf-ui-bg-muted)] text-[var(--zf-ui-text-muted)]"
                      )}>
                        {comp.installed ? "installed" : "available"}
                      </span>
                    </div>
                    <div className="mt-3 flex items-center justify-between">
                      <code className="text-[10px] text-[var(--zf-ui-text-muted)] font-mono">{comp.filename}</code>
                      <code className="rounded bg-[var(--zf-ui-bg-muted)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--zf-ui-text-soft)]">
                        {"@" + "/shared/ui/" + comp.name}
                      </code>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))
        )}

        {/* Install instructions */}
        <section className="rounded-lg border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg-subtle)] p-6">
          <h2 className="mb-3 text-sm font-semibold text-[var(--zf-ui-text)]">How to Install</h2>
          <div className="space-y-3 text-sm text-[var(--zf-ui-text-soft)]">
            <p><strong className="text-[var(--zf-ui-text)]">Via MCP agent:</strong></p>
            <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">{`install_ui_components(names=["button","card","dialog"])`}</pre>
            <p><strong className="text-[var(--zf-ui-text)]">Via API:</strong></p>
            <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">{`POST /api/projects/{owner}/{project}/install/ui
{ "names": ["button", "card"], "overwrite": false }`}</pre>
            <p><strong className="text-[var(--zf-ui-text)]">After install, use in TSX templates:</strong></p>
            <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">{"import { Button } from " + '"@/shared/ui/button"' + "\nimport { Card, CardHeader, CardContent } from " + '"@/shared/ui/card"'}</pre>
          </div>
        </section>
      </div>
    </div>
  );
}
