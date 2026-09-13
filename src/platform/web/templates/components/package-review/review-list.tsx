/**
 * One named list out of a review, rendered whole.
 *
 * `tone="danger"` marks a list whose contents are effects on things the user
 * already had. It is deliberately weaker than a violation: it colours a heading,
 * it does not claim anything is blocked.
 */
export default function ReviewList({ title, items, tone = "plain", emptyNote = "None" }) {
  const values = Array.isArray(items) ? items.filter((item) => item !== null && item !== undefined) : [];
  return (
    <div
      className={
        tone === "danger"
          ? "rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2"
          : "rounded-lg border border-border bg-popover px-3 py-2"
      }
    >
      <p className="m-0 flex items-baseline justify-between gap-2 text-[0.7rem] font-mono uppercase tracking-widest text-muted-foreground">
        <span>{title}</span>
        <span>{values.length}</span>
      </p>
      {values.length ? (
        <ul className="m-0 mt-1.5 list-none space-y-0.5 p-0 font-mono text-[0.72rem] text-foreground">
          {values.map((item, index) => (
            <li key={`${title}-${index}`} className="break-all">{String(item)}</li>
          ))}
        </ul>
      ) : (
        <p className="m-0 mt-1 text-xs text-muted-foreground">{emptyNote}</p>
      )}
    </div>
  );
}
