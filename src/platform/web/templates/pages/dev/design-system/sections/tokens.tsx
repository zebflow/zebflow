import { SectionHeading, SubHeading, Demo, SampleCode, TokenSwatch } from "@/pages/dev/design-system/components/gallery";

/**
 * The theme, as the page currently resolves it. Toggle the theme in the
 * header and every swatch changes — that is the whole point of naming a
 * role instead of a colour. Contract: docs/contracts/kinds/ui-theme.
 */

const PAIRS = [
  ["background", "foreground"],
  ["card", "card-foreground"],
  ["popover", "popover-foreground"],
  ["primary", "primary-foreground"],
  ["secondary", "secondary-foreground"],
  ["muted", "muted-foreground"],
  ["accent", "accent-foreground"],
  ["destructive", "destructive-foreground"],
  ["success", "success-foreground"],
  ["warning", "warning-foreground"],
  ["info", "info-foreground"],
];

const SINGLES = ["border", "input", "ring"];
const CHARTS = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"];
const SIDEBAR = [
  ["sidebar", "sidebar-foreground"],
  ["sidebar-primary", "sidebar-primary-foreground"],
  ["sidebar-accent", "sidebar-accent-foreground"],
  ["sidebar-border", null],
  ["sidebar-ring", null],
];

export default function TokensSection() {
  return (
    <div>
      <SectionHeading
        title="Theme"
        description="shadcn/ui token names, verbatim, with Zebflow's values. A component names a role; :root and .dark in styles/main.css decide the colour. Utilities: bg-<token>, text-<token>, border-<token>, ring-<token>, with /<alpha>."
      />

      <SubHeading title="Pairs — a surface and what sits on it" />
      <Demo>
        <div className="grid grid-cols-1 gap-x-8 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
          {PAIRS.map(([bg, fg]) => (
            <TokenSwatch key={bg} token={bg} foreground={fg} />
          ))}
        </div>
      </Demo>
      <SampleCode code={`<div className="bg-card text-card-foreground border border-border">…</div>
<button className="bg-primary text-primary-foreground hover:bg-primary/90">Save</button>
<p className="text-muted-foreground">secondary text</p>`} />

      <SubHeading title="Lines and focus" />
      <Demo>
        <div className="grid grid-cols-1 gap-x-8 gap-y-4 sm:grid-cols-3">
          {SINGLES.map((t) => (
            <TokenSwatch key={t} token={t} />
          ))}
        </div>
        <div className="mt-6 flex flex-wrap items-center gap-4">
          <div className="rounded-md border border-border px-3 py-2 text-sm">border</div>
          <div className="rounded-md border border-input px-3 py-2 text-sm">input</div>
          <div className="rounded-md border border-input px-3 py-2 text-sm ring-2 ring-ring/40">ring-2 ring-ring/40</div>
        </div>
      </Demo>

      <SubHeading title="Status" />
      <Demo>
        <div className="flex flex-wrap gap-3">
          {["destructive", "success", "warning", "info"].map((t) => (
            <div key={t} className={`rounded-md border px-3 py-2 text-sm border-${t}/30 bg-${t}/10 text-${t}`}>
              {t}
            </div>
          ))}
          <span hidden tw-variants="border-destructive/30 bg-destructive/10 text-destructive border-success/30 bg-success/10 text-success border-warning/30 bg-warning/10 text-warning border-info/30 bg-info/10 text-info" />
        </div>
        <div className="mt-4 flex flex-wrap gap-3">
          {["destructive", "success", "warning", "info"].map((t) => (
            <div key={t} className={`rounded-md px-3 py-2 text-sm bg-${t} text-${t}-foreground`}>
              solid {t}
            </div>
          ))}
          <span hidden tw-variants="bg-destructive text-destructive-foreground bg-success text-success-foreground bg-warning text-warning-foreground bg-info text-info-foreground" />
        </div>
      </Demo>
      <SampleCode code={`// tinted (default for inline notices)
<div className="border border-destructive/30 bg-destructive/10 text-destructive">…</div>
// solid (a filled badge or button)
<div className="bg-success text-success-foreground">…</div>`} />

      <SubHeading title="Charts" />
      <Demo>
        <div className="flex items-end gap-3">
          {CHARTS.map((t, i) => (
            <div key={t} className="flex flex-col items-center gap-2">
              <div className="w-10 rounded-t-md" style={{ height: `${40 + i * 18}px`, background: `var(--${t})` }} />
              <span className="font-mono text-[0.65rem] text-muted-foreground">{t}</span>
            </div>
          ))}
        </div>
      </Demo>

      <SubHeading title="Sidebar" />
      <Demo className="p-0">
        <div className="flex">
          <aside className="w-48 shrink-0 space-y-1 border-r border-sidebar-border bg-sidebar p-3 text-sidebar-foreground">
            <div className="rounded-md bg-sidebar-primary px-2.5 py-1.5 text-sm text-sidebar-primary-foreground">active</div>
            <div className="rounded-md bg-sidebar-accent px-2.5 py-1.5 text-sm text-sidebar-accent-foreground">hover</div>
            <div className="rounded-md px-2.5 py-1.5 text-sm">rest</div>
          </aside>
          <div className="flex-1 p-6">
            <div className="grid grid-cols-1 gap-x-8 gap-y-4 sm:grid-cols-2">
              {SIDEBAR.map(([bg, fg]) => (
                <TokenSwatch key={bg} token={bg} foreground={fg} />
              ))}
            </div>
          </div>
        </div>
      </Demo>

      <SubHeading title="Typography" />
      <Demo>
        <div className="space-y-3">
          <p className="font-display text-3xl font-bold tracking-tight text-foreground">Display — Space Grotesk</p>
          <p className="font-sans text-base text-foreground">Sans — the body face. The quick brown fox jumps over the lazy dog.</p>
          <p className="font-mono text-sm text-foreground">Mono — JetBrains Mono · const x = 42;</p>
          <p className="text-sm text-muted-foreground">Secondary — text-muted-foreground</p>
        </div>
      </Demo>

      <SubHeading title="Radius" />
      <Demo>
        <div className="flex flex-wrap items-end gap-4">
          {["rounded-sm", "rounded-md", "rounded-lg", "rounded-xl", "rounded-2xl", "rounded-full"].map((r) => (
            <div key={r} className="flex flex-col items-center gap-2">
              <div className={`h-12 w-12 border border-border bg-muted ${r}`} />
              <span className="font-mono text-[0.65rem] text-muted-foreground">{r}</span>
            </div>
          ))}
          <span hidden tw-variants="rounded-sm rounded-md rounded-lg rounded-xl rounded-2xl rounded-full" />
        </div>
      </Demo>
    </div>
  );
}
