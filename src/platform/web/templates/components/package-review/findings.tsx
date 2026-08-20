/**
 * Violations and warnings are different in kind, so they are different in shape.
 *
 * A warning is an effect the user may accept and install anyway. A violation
 * means the package will not be installed, whoever approves it — there is no
 * control anywhere in this UI that overrides one, and a user facing a refusal
 * has to be able to read why.
 */

export function ViolationNotice({ items, subject = "This package" }) {
  const values = Array.isArray(items) ? items.filter(Boolean) : [];
  if (!values.length) return null;
  return (
    <section className="rounded-lg border-2 border-red-600 bg-red-600/10 p-3" role="alert">
      <div className="flex flex-wrap items-center gap-2">
        <span className="inline-flex items-center rounded-sm bg-red-600 px-2 py-0.5 font-mono text-[0.7rem] font-bold uppercase tracking-widest text-white">
          Blocked
        </span>
        <p className="m-0 text-sm font-semibold text-red-700">
          {subject} will not be installed
        </p>
      </div>
      <p className="m-0 mt-2 text-xs text-red-700">
        {values.length === 1 ? "This is a violation" : `These are ${values.length} violations`}, not a
        warning. A violation cannot be accepted or overridden — not by you, not by an administrator.
        The install refuses for exactly the reason{values.length === 1 ? "" : "s"} below.
      </p>
      <ul className="m-0 mt-2 list-none space-y-1 p-0">
        {values.map((item, index) => (
          <li
            key={`violation-${index}`}
            className="rounded-md border border-red-600/50 bg-red-600/10 px-2.5 py-1.5 font-mono text-xs text-red-800"
          >
            {String(item)}
          </li>
        ))}
      </ul>
    </section>
  );
}

export function WarningNotice({ items }) {
  const values = Array.isArray(items) ? items.filter(Boolean) : [];
  if (!values.length) return null;
  return (
    <section className="rounded-lg border border-dashed border-amber-500/70 bg-amber-500/10 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="inline-flex items-center rounded-sm border border-amber-600/60 px-2 py-0.5 font-mono text-[0.7rem] font-semibold uppercase tracking-widest text-amber-800">
          Accept or cancel
        </span>
        <p className="m-0 text-sm font-medium text-amber-900">
          {values.length} warning{values.length === 1 ? "" : "s"}
        </p>
      </div>
      <p className="m-0 mt-2 text-xs text-amber-800">
        Warnings do not block anything. They are effects you are being told about so that installing
        is a decision rather than a surprise. Installing accepts them.
      </p>
      <ul className="m-0 mt-2 list-disc space-y-1 pl-5 text-xs text-amber-900">
        {values.map((item, index) => <li key={`warning-${index}`}>{String(item)}</li>)}
      </ul>
    </section>
  );
}

export default ViolationNotice;
