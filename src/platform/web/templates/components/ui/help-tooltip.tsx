import { createPortal, useEffect, useId, useLayoutEffect, useRef, useState } from "zeb";

export default function HelpTooltip({ text }) {
  const buttonRef = useRef(null);
  const tooltipRef = useRef(null);
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [rect, setRect] = useState(null);
  const tooltipId = useId();
  const visible = open || hovered || focused;

  useEffect(() => {
    if (!visible) return;
    const onKeyDown = (event) => {
      if (event.key === "Escape") setOpen(false);
    };
    const onPointerDown = (event) => {
      const target = event.target;
      if (
        !(target instanceof Element) ||
        (!target.closest("[data-help-tooltip-root]") && !target.closest("[data-help-tooltip-portal]"))
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("pointerdown", onPointerDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("pointerdown", onPointerDown);
    };
  }, [visible]);

  useLayoutEffect(() => {
    if (!visible || !buttonRef.current || typeof window === "undefined") return;

    const updateRect = () => {
      setRect(buttonRef.current.getBoundingClientRect());
    };

    updateRect();
    window.addEventListener("resize", updateRect);
    window.addEventListener("scroll", updateRect, true);
    return () => {
      window.removeEventListener("resize", updateRect);
      window.removeEventListener("scroll", updateRect, true);
    };
  }, [visible]);

  useEffect(() => {
    const tooltipEl = tooltipRef.current;
    if (!tooltipEl) return;

    const canUsePopover =
      typeof tooltipEl.showPopover === "function" &&
      typeof tooltipEl.hidePopover === "function";
    if (!canUsePopover) return;

    try {
      if (visible && rect && !tooltipEl.matches(":popover-open")) {
        tooltipEl.showPopover();
      } else if ((!visible || !rect) && tooltipEl.matches(":popover-open")) {
        tooltipEl.hidePopover();
      }
    } catch (_) {
      // Popover can throw during rapid open/close races; the next render will resync it.
    }

    return () => {
      try {
        if (tooltipEl.matches(":popover-open")) tooltipEl.hidePopover();
      } catch (_) {}
    };
  }, [visible, rect]);

  const tooltip = visible && rect && typeof document !== "undefined"
    ? createPortal(
        <span
          ref={tooltipRef}
          id={tooltipId}
          popover="manual"
          data-help-tooltip-portal="true"
          className="pointer-events-none fixed z-[1000] min-w-40 max-w-60 rounded-md border border-ui-border bg-gray-800 px-2.5 py-1.5 text-left text-[11px] font-normal leading-[1.5] tracking-normal text-gray-100 shadow-lg"
          style={{
            left: `${Math.min(Math.max(rect.left + rect.width / 2, 132), window.innerWidth - 132)}px`,
            top: `${rect.top - 8}px`,
            margin: "0",
            transform: "translate(-50%, -100%)",
          }}
          role="tooltip"
        >
          {text}
          <span className="absolute left-1/2 top-full h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[5px] border-x-transparent border-t-gray-800" />
        </span>,
        document.body
      )
    : null;

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        data-help-tooltip-root="true"
        className="group relative inline-flex items-center rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-ui-primary/30"
        aria-label={text}
        aria-expanded={visible ? "true" : "false"}
        aria-describedby={visible ? tooltipId : undefined}
        onClick={() => setOpen((value) => !value)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onFocus={() => setFocused(true)}
        onBlur={(event) => {
          const next = event.relatedTarget;
          if (!(next instanceof Element) || !event.currentTarget.contains(next)) {
            setFocused(false);
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
      </button>
      {tooltip}
    </>
  );
}
