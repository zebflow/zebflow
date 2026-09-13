export const page = {
  head: { title: "Color Token Test" },
  body: { className: "font-sans bg-background text-foreground min-h-screen p-8" },
};

export default function Page() {
  return (
<Page>
  <div className="max-w-2xl mx-auto flex flex-col gap-6">
    <h1 className="text-2xl font-bold text-foreground">Color Token Test</h1>

    {/* Backgrounds */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">Backgrounds</p>
      <div className="flex gap-2 flex-wrap">
        <Swatch cls="bg-background" label="bg-background" />
        <Swatch cls="bg-card" label="bg-card" />
        <Swatch cls="bg-muted" label="bg-muted" />
        <Swatch cls="bg-accent" label="bg-accent" />
      </div>
    </section>

    {/* Text */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">Text</p>
      <div className="flex flex-col gap-1">
        <p className="text-foreground text-sm">text-foreground — full contrast</p>
        <p className="text-muted-foreground text-sm">text-muted-foreground — secondary</p>
        <p className="text-muted-foreground text-sm">text-muted-foreground — muted</p>
        <p className="text-primary text-sm">text-primary — accent color</p>
      </div>
    </section>

    {/* Borders */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">Borders</p>
      <div className="flex gap-2 flex-wrap">
        <div className="px-4 py-2 rounded border border-border text-foreground text-sm">border-border</div>
        <div className="px-4 py-2 rounded border border-border text-foreground text-sm">border-border</div>
      </div>
    </section>

    {/* Brand */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">Brand</p>
      <div className="flex gap-2 flex-wrap">
        <Swatch cls="bg-primary" label="bg-primary" />
        <Swatch cls="bg-info" label="bg-info" textCls="text-white" />
        <Swatch cls="bg-primary" label="bg-primary" />
        <Swatch cls="bg-info" label="bg-info" textCls="text-white" />
      </div>
    </section>
  </div>
</Page>
  );
}

function Swatch({ cls, label, textCls = "text-foreground" }: { cls: string; label: string; textCls?: string }) {
  return (
    <div className={`${cls} border border-border rounded px-3 py-2 text-xs font-mono ${textCls}`}>
      {label}
    </div>
  );
}
