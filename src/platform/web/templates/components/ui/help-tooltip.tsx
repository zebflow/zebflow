import { useEffect, useId, useState } from "zeb";

export default function HelpTooltip({ text }) {
  const [open, setOpen] = useState(false);
  const tooltipId = useId();

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event) => {
      if (event.key === "Escape") setOpen(false);
    };
    const onPointerDown = (event) => {
      const target = event.target;
      if (!(target instanceof Element) || !target.closest("[data-help-tooltip-root]")) {
        setOpen(false);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("pointerdown", onPointerDown);
    };
  }, [open]);

  return (
    <button
      type="button"
      data-help-tooltip-root="true"
      className="group relative inline-flex items-center rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-ui-primary/30"
      aria-label={text}
      aria-expanded={open ? "true" : "false"}
      aria-describedby={open ? tooltipId : undefined}
      onClick={() => setOpen((value) => !value)}
      onBlur={(event) => {
        const next = event.relatedTarget;
        if (!(next instanceof Element) || !event.currentTarget.contains(next)) {
          setOpen(false);
        }
      }}
    >
      <span
        className="inline-flex text-ui-text-muted transition-colors duration-150 group-hover:text-ui-text group-focus-within:text-ui-text"
        aria-hidden="true"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none">
          <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.8"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
          <circle cx="12" cy="17" r="0.5" fill="currentColor" stroke="currentColor" strokeWidth="1.5"/>
        </svg>
      </span>
      <span
        id={tooltipId}
        className={`pointer-events-none absolute bottom-[calc(100%+8px)] left-1/2 z-50 min-w-40 max-w-60 -translate-x-1/2 rounded-md border border-ui-border bg-gray-800 px-2.5 py-1.5 text-left text-[11px] font-normal leading-[1.5] tracking-normal text-gray-100 shadow-lg transition-[opacity,visibility] duration-150 ${
          open
            ? "visible opacity-100"
            : "invisible opacity-0 group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100"
        }`}
        role="tooltip"
      >
        {text}
        <span className="absolute left-1/2 top-full h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[5px] border-x-transparent border-t-gray-800" />
      </span>
    </button>
  );
}
