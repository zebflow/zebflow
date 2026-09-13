import { cx, createContext, useContext, useRef, useState, useEffect, createPortal } from "zeb/react";
import { useAnchoredPosition, useEscape } from "zeb/ui/hooks";

/**
 * HoverCard — shadcn's hover-card, on Zebflow's engine. `HoverCard`/
 * `HoverCardTrigger`/`HoverCardContent` compose through `createContext`
 * instead of `child.type` detection, the same shape as `zeb/ui/tooltip`:
 * `HoverCardTrigger` wraps its children in a plain `<span>` that owns the
 * hover timer and anchors the panel, without touching the trigger child's
 * own props. Opens after 700ms of hovering (upstream's default), closes
 * 150ms after the pointer leaves both the trigger and the panel — the delay
 * (and the panel's own `onMouseEnter` cancelling it) is the hover bridge
 * that lets the pointer travel from trigger to panel without the card
 * closing first. Escape also dismisses it. `HoverCardContent` reads its own
 * `side`/`align`/`sideOffset` (defaulting to `bottom`/`center`/`8`) and
 * portals to `document.body` in the browser, rendering inline during SSR.
 */

const HoverCardContext = createContext(null);

function useHoverCardContext(name) {
  const ctx = useContext(HoverCardContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <HoverCard>`);
  return ctx;
}

export function HoverCard({ openDelay = 700, closeDelay = 150, children }) {
  const [isOpen, setIsOpen] = useState(false);
  const anchorRef = useRef(null);
  const panelRef = useRef(null);
  const timerRef = useRef(null);

  useEscape(isOpen, () => setIsOpen(false));

  useEffect(() => () => {
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  function show() {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setIsOpen(true), openDelay);
  }
  function hide() {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setIsOpen(false), closeDelay);
  }
  function cancelHide() {
    if (timerRef.current) clearTimeout(timerRef.current);
  }

  return (
    <HoverCardContext.Provider value={{ isOpen, show, hide, cancelHide, anchorRef, panelRef }}>{children}</HoverCardContext.Provider>
  );
}

export function HoverCardTrigger({ className, children, ...props }) {
  const { anchorRef, show, hide } = useHoverCardContext("HoverCardTrigger");
  return (
    <span ref={anchorRef} data-slot="hover-card-trigger" className={cx("inline-flex", className)} onMouseEnter={show} onMouseLeave={hide} {...props}>
      {children}
    </span>
  );
}

export function HoverCardContent({ className, children, side = "bottom", align = "center", sideOffset = 8, ...props }) {
  const { isOpen, anchorRef, panelRef, cancelHide, hide } = useHoverCardContext("HoverCardContent");
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, { side, align, offset: sideOffset });

  if (!isOpen) return null;

  const node = (
    <div ref={panelRef} className="fixed z-50" style={{ top: `${top}px`, left: `${left}px` }} onMouseEnter={cancelHide} onMouseLeave={hide}>
      <div
        data-slot="hover-card-content"
        className={cx("w-64 rounded-md border border-border bg-popover p-4 text-popover-foreground shadow-md outline-none", className)}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export default HoverCard;
