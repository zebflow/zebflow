/** One part of the fence form: a plain heading, the sentence that explains it, the inputs. */
export default function ControlSection({ title, hint, children }) {
  return (
    <section className="grid gap-2 rounded-md border border-border p-3">
      <div>
        <p className="text-sm font-medium text-foreground">{title}</p>
        {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
      </div>
      {children}
    </section>
  );
}
