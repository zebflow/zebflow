export const page = {
  head: { title: "Color Token Test" },
  body: { className: "font-sans bg-bg text-body min-h-screen p-8" },
};

export default function Page() {
  return (
<Page>
  <div className="max-w-2xl mx-auto flex flex-col gap-6">
    <h1 className="text-2xl font-bold text-body">Color Token Test</h1>

    {/* Backgrounds */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-body-muted uppercase tracking-wider">Backgrounds</p>
      <div className="flex gap-2 flex-wrap">
        <Swatch cls="bg-bg" label="bg-bg" />
        <Swatch cls="bg-surface" label="bg-surface" />
        <Swatch cls="bg-surface-2" label="bg-surface-2" />
        <Swatch cls="bg-surface-3" label="bg-surface-3" />
      </div>
    </section>

    {/* Text */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-body-muted uppercase tracking-wider">Text</p>
      <div className="flex flex-col gap-1">
        <p className="text-body text-sm">text-body — full contrast</p>
        <p className="text-body-soft text-sm">text-body-soft — secondary</p>
        <p className="text-body-muted text-sm">text-body-muted — muted</p>
        <p className="text-accent text-sm">text-accent — accent color</p>
      </div>
    </section>

    {/* Borders */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-body-muted uppercase tracking-wider">Borders</p>
      <div className="flex gap-2 flex-wrap">
        <div className="px-4 py-2 rounded border border-border text-body text-sm">border-border</div>
        <div className="px-4 py-2 rounded border border-border-soft text-body text-sm">border-border-soft</div>
      </div>
    </section>

    {/* Brand */}
    <section className="flex flex-col gap-2">
      <p className="text-xs text-body-muted uppercase tracking-wider">Brand</p>
      <div className="flex gap-2 flex-wrap">
        <Swatch cls="bg-brand-orange" label="bg-brand-orange" />
        <Swatch cls="bg-brand-blue" label="bg-brand-blue" textCls="text-white" />
        <Swatch cls="bg-accent" label="bg-accent" />
        <Swatch cls="bg-accent-alt" label="bg-accent-alt" textCls="text-white" />
      </div>
    </section>
  </div>
</Page>
  );
}

function Swatch({ cls, label, textCls = "text-body" }: { cls: string; label: string; textCls?: string }) {
  return (
    <div className={`${cls} border border-border rounded px-3 py-2 text-xs font-mono ${textCls}`}>
      {label}
    </div>
  );
}
