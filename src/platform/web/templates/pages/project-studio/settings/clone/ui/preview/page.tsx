import { cx, useState, Link } from "zeb";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardContent from "@/components/ui/card-content";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export const app = {};

const CATEGORY_LABELS: Record<string, string> = {
  primitives: "Primitives",
  display: "Display",
  layout: "Layout",
  navigation: "Navigation",
  overlay: "Overlay",
  complex: "Complex",
};

const CATEGORY_ORDER = ["primitives", "display", "layout", "navigation", "overlay", "complex"];

export default function Page(input: any) {
  const components: any[] = Array.isArray(input?.components) ? input.components : [];
  const [activeCategory, setActiveCategory] = useState("all");
  const [search, setSearch] = useState("");
  const settingsHref = `${input?.project_href ?? ""}/settings`;

  const categories = CATEGORY_ORDER.filter((cat) =>
    components.some((c) => c.category === cat)
  );

  const filtered = components.filter((c) => {
    const matchCat = activeCategory === "all" || c.category === activeCategory;
    const matchSearch =
      !search ||
      c.name.toLowerCase().includes(search.toLowerCase()) ||
      c.description.toLowerCase().includes(search.toLowerCase());
    return matchCat && matchSearch;
  });

  const grouped = CATEGORY_ORDER.reduce<Record<string, any[]>>((acc, cat) => {
    const items = filtered.filter((c) => c.category === cat);
    if (items.length > 0) acc[cat] = items;
    return acc;
  }, {});

  const total = components.length;

  return (
    <ProjectStudioShell
      projectHref={input?.project_href}
      projectLabel={input?.title}
      currentMenu="Settings"
      owner={input?.owner ?? ""}
      project={input?.project ?? ""}
      nav={input?.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <nav className="shrink-0 flex items-center gap-2 border-b border-[var(--studio-border)] bg-[var(--studio-panel)] px-3 py-2 text-[0.78rem]">
          <Link href={settingsHref} className="text-[var(--studio-accent)] hover:underline">
            Settings
          </Link>
          <span className="text-[var(--studio-text-soft)]">/</span>
          <span className="text-[var(--studio-text-soft)]">Clone</span>
          <span className="text-[var(--studio-text-soft)]">/</span>
          <span className="font-medium text-[var(--studio-text)]">UI preview</span>
        </nav>

        <div className="flex-1 min-h-0 overflow-y-auto">
          <div className="border-b border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg-subtle)] px-6 py-5">
            <div className="mx-auto max-w-6xl">
              <div className="flex items-start justify-between">
                <div>
                  <h1 className="text-xl font-semibold">UI Component Catalog</h1>
                  <p className="mt-1 text-sm text-[var(--zf-ui-text-muted)]">
                    shadcn-compatible Zeb React components — install into{" "}
                    <code className="rounded bg-[var(--zf-ui-bg-muted)] px-1 py-0.5 font-mono text-xs">
                      shared/ui/
                    </code>
                  </p>
                </div>
                <div className="flex items-center gap-3 text-sm text-[var(--zf-ui-text-muted)]">
                  <span>
                    <strong className="text-[var(--zf-ui-text)]">{total}</strong> components
                  </span>
                  <span>·</span>
                  <span>
                    <strong className="text-[var(--zf-ui-text)]">{Object.keys(CATEGORY_LABELS).length}</strong>{" "}
                    categories
                  </span>
                </div>
              </div>

              <div className="mt-4 flex flex-wrap items-center gap-3">
                <Input
                  type="search"
                  placeholder="Search components…"
                  value={search}
                  onChange={(e: any) => setSearch(e.target.value)}
                  className="h-8 w-64"
                />
                <div className="flex items-center gap-1">
                  <Button
                    size="xs"
                    variant={activeCategory === "all" ? "primary" : "outline"}
                    className="rounded-full"
                    onClick={() => setActiveCategory("all")}
                  >
                    All ({total})
                  </Button>
                  {categories.map((cat) => {
                    const count = components.filter((c) => c.category === cat).length;
                    return (
                      <Button
                        key={cat}
                        size="xs"
                        variant={activeCategory === cat ? "primary" : "outline"}
                        className="rounded-full capitalize"
                        onClick={() => setActiveCategory(cat)}
                      >
                        {CATEGORY_LABELS[cat] ?? cat} ({count})
                      </Button>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>

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
                      <Card
                        key={comp.name}
                        className="group p-4 hover:border-[var(--zf-color-brand-blue)]/40 transition-colors"
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div>
                            <p className="text-sm font-semibold">{comp.name}</p>
                            <p className="mt-0.5 text-xs text-[var(--zf-ui-text-muted)]">{comp.description}</p>
                          </div>
                          <Badge
                            variant={comp.installed ? undefined : "secondary"}
                            className={cx(
                              "text-[10px] shrink-0",
                              comp.installed && "bg-green-500/10 text-green-600 border-transparent"
                            )}
                          >
                            {comp.installed ? "installed" : "available"}
                          </Badge>
                        </div>
                        <div className="mt-3 flex items-center justify-between">
                          <code className="text-[10px] text-[var(--zf-ui-text-muted)] font-mono">{comp.filename}</code>
                          <code className="rounded bg-[var(--zf-ui-bg-muted)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--zf-ui-text-soft)]">
                            {"@" + "/shared/ui/" + comp.name}
                          </code>
                        </div>
                      </Card>
                    ))}
                  </div>
                </section>
              ))
            )}

            <Card>
              <CardHeader>
                <h2 className="text-sm font-semibold">How to Install</h2>
              </CardHeader>
              <CardContent className="space-y-3 text-sm text-[var(--zf-ui-text-soft)]">
                <p>
                  <strong className="text-[var(--zf-ui-text)]">Via MCP agent:</strong>
                </p>
                <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">{`install_ui_components(names=["button","card","dialog"])`}</pre>
                <p>
                  <strong className="text-[var(--zf-ui-text)]">Via API:</strong>
                </p>
                <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">{`POST /api/projects/${input?.owner ?? "{owner}"}/${input?.project ?? "{project}"}/install/ui
{ "names": ["button", "card"], "overwrite": false }`}</pre>
                <p>
                  <strong className="text-[var(--zf-ui-text)]">After install, use in TSX templates:</strong>
                </p>
                <pre className="rounded-md bg-[var(--zf-ui-bg-muted)] px-4 py-3 font-mono text-xs overflow-x-auto">
                  {'import { Button } from "@/shared/ui/button"\nimport { Card, CardHeader, CardContent } from "@/shared/ui/card"'}
                </pre>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </ProjectStudioShell>
  );
}
