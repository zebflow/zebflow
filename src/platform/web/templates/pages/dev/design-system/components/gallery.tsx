import { cx } from "zeb/react";
import { CodeBlock } from "zeb/ui/code-block";

/**
 * The scaffolding every gallery section is built from. A section is a list
 * of `Entry`s; an entry is a heading, a live demo on a card, and the code
 * that produced it. Nothing here is a primitive — the primitives live in
 * `components/ui/` and this page only shows them.
 */

export function SectionHeading({ title, description }) {
  return (
    <div className="mb-8">
      <h2 className="text-2xl font-bold tracking-tight text-foreground">{title}</h2>
      {description ? <p className="mt-1.5 max-w-2xl text-sm text-muted-foreground">{description}</p> : null}
      <div className="mt-5 h-px bg-border" />
    </div>
  );
}

export function SubHeading({ title }) {
  return (
    <h3 className="mb-4 mt-10 text-[0.68rem] font-bold uppercase tracking-widest text-muted-foreground first:mt-0">
      {title}
    </h3>
  );
}

/** A live demo sits on a card so surface tokens are visible against the page. */
export function Demo({ children, className }) {
  return <div className={cx("rounded-xl border border-border bg-card p-6 text-card-foreground", className)}>{children}</div>;
}

/** The code that produced a demo — zeb/ui's own CodeBlock, so the gallery eats its own cooking. */
export function SampleCode({ code, language = "tsx" }) {
  return <CodeBlock className="mt-3" language={language} code={code} />;
}

/**
 * One component. `file` is the path under `components/ui/` — the guard in
 * tests/rwe reads these to prove every primitive is on the page.
 */
export function Entry({ name, file, description, children, code, demoClassName }) {
  return (
    <div className="mb-12" data-gallery-entry={file}>
      <div className="mb-3 flex items-start gap-4">
        <div className="flex-1">
          <h3 className="text-base font-semibold text-foreground">{name}</h3>
          {description ? <p className="mt-0.5 text-sm text-muted-foreground">{description}</p> : null}
        </div>
        <code className="shrink-0 whitespace-nowrap rounded border border-border bg-muted px-2 py-1 font-mono text-[0.68rem] text-muted-foreground">
          {file.includes("/") ? file : `ui/${file}`}
        </code>
      </div>
      <Demo className={demoClassName}>{children}</Demo>
      {code ? <SampleCode code={code} /> : null}
    </div>
  );
}

/** A variant × size grid: one row per variant, one column per size. */
export function Matrix({ rows, cols, render, rowLabel = "variant", colLabel = "size" }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            <th className="pb-2 pr-4 text-left font-mono text-[0.65rem] font-normal uppercase tracking-wider text-muted-foreground">
              {rowLabel} \ {colLabel}
            </th>
            {cols.map((c) => (
              <th key={c} className="pb-2 pr-4 text-left font-mono text-[0.65rem] font-normal uppercase tracking-wider text-muted-foreground">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r} className="border-t border-border">
              <td className="py-2.5 pr-4 font-mono text-xs text-muted-foreground">{r}</td>
              {cols.map((c) => (
                <td key={c} className="py-2.5 pr-4 align-middle">
                  {render(r, c)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** A theme token as it currently resolves, with its name. */
export function TokenSwatch({ token, foreground }) {
  return (
    <div className="flex items-center gap-3">
      <span
        className="h-9 w-9 shrink-0 rounded-md border border-border"
        style={{ background: `var(--${token})` }}
      >
        {foreground ? (
          <span className="flex h-full w-full items-center justify-center font-mono text-[0.7rem]" style={{ color: `var(--${foreground})` }}>
            Aa
          </span>
        ) : null}
      </span>
      <div className="min-w-0">
        <div className="truncate font-mono text-xs text-foreground">--{token}</div>
        {foreground ? <div className="truncate font-mono text-[0.65rem] text-muted-foreground">--{foreground}</div> : null}
      </div>
    </div>
  );
}
