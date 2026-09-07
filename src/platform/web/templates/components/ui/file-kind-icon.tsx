/**
 * What a file is, at a glance.
 *
 * Real technology marks where there is one — the devicon set the studio already
 * loads — and a plain document otherwise. This lived twice, identically, in the
 * registry editor and the pipelines page; both now read it from here.
 */
export default function FileKindIcon({ name = "" }) {
  const lower = String(name).toLowerCase();

  // A pipeline definition is a `.zf.json`, which by extension alone would look
  // like any other data file.
  if (lower.endsWith(".zf.json")) {
    return (
      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4 text-dark-accent1" aria-hidden="true">
        <circle cx="6" cy="6" r="2.2" stroke="currentColor" strokeWidth="1.6" />
        <circle cx="18" cy="12" r="2.2" stroke="currentColor" strokeWidth="1.6" />
        <circle cx="6" cy="18" r="2.2" stroke="currentColor" strokeWidth="1.6" />
        <path d="M8.2 6H12a2 2 0 0 1 2 2v2M8.2 18H12a2 2 0 0 0 2-2v-2" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      </svg>
    );
  }

  const ext = (lower.split(".").pop() ?? "").toLowerCase();
  if (ext === "tsx" || ext === "jsx") {
    return <i className="devicon-react-original colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  if (ext === "ts") {
    return <i className="devicon-typescript-plain colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  if (ext === "css" || ext === "scss") {
    return <i className="devicon-css3-plain colored text-[0.95rem] leading-none" aria-hidden="true" />;
  }
  if (ext === "json" || ext === "yaml" || ext === "yml") {
    return (
      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
        <path d="M9 4H7a2 2 0 0 0-2 2v3a2 2 0 0 1-2 2 2 2 0 0 1 2 2v3a2 2 0 0 0 2 2h2M15 4h2a2 2 0 0 1 2 2v3a2 2 0 0 0 2 2 2 2 0 0 0-2 2v3a2 2 0 0 1-2 2h-2" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  if (ext === "md" || ext === "txt") {
    return (
      <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
        <path d="M14 2v6h6M8 13h6M8 17h4" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round"/>
      <path d="M14 2v6h6" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round"/>
    </svg>
  );
}
