import { cx, createContext, useContext, useRef, useState, useId, useEffect, createPortal } from "zeb/react";
import { useAnchoredPosition, useEscape } from "zeb/ui/hooks";

/**
 * Tooltip — shadcn's tooltip, on Zebflow's engine.
 *
 * `Tooltip`/`TooltipTrigger`/`TooltipContent` compose through `createContext`
 * instead of `child.type` detection. Dropped versus upstream:
 * `TooltipProvider` — it exists only to share one `delayDuration` through
 * context, and each `Tooltip` here just takes its own `delayDuration`
 * (default 300ms) instead.
 *
 * `TooltipTrigger` wraps its children in a plain `<span>` that owns the
 * hover/focus timers and anchors the panel, without touching the trigger
 * child's own props; it also carries `aria-describedby` (via `useId()`)
 * while the tooltip is open, and Escape dismisses it. Hiding is delayed
 * (~100ms) and cancelled if the pointer reaches `TooltipContent` itself —
 * without that, leaving the trigger's border box on the way to the panel
 * (which sits a few pixels away) would hide the tooltip before the pointer
 * could arrive, which both fails WCAG 1.4.13 and upstream's "move the
 * pointer onto the tooltip" behaviour. `TooltipContent` reads its own
 * `side`/`align`/`sideOffset` (defaulting to `top`/`center`/`6`, matching
 * upstream) and portals to `document.body` in the browser, rendering inline
 * during SSR.
 */

const TooltipContext = createContext(null);

function useTooltipContext(name) {
  const ctx = useContext(TooltipContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Tooltip>`);
  return ctx;
}

export function Tooltip({ delayDuration = 300, children }) {
  const [isOpen, setIsOpen] = useState(false);
  const anchorRef = useRef(null);
  const panelRef = useRef(null);
  const showTimer = useRef(null);
  const hideTimer = useRef(null);
  const descriptionId = useId();

  useEscape(isOpen, () => setIsOpen(false));

  useEffect(
    () => () => {
      if (showTimer.current) clearTimeout(showTimer.current);
      if (hideTimer.current) clearTimeout(hideTimer.current);
    },
    []
  );

  function show() {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
    if (showTimer.current) clearTimeout(showTimer.current);
    showTimer.current = setTimeout(() => setIsOpen(true), delayDuration);
  }
  function scheduleHide() {
    if (showTimer.current) {
      clearTimeout(showTimer.current);
      showTimer.current = null;
    }
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => setIsOpen(false), 100);
  }
  function hideNow() {
    if (showTimer.current) clearTimeout(showTimer.current);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    setIsOpen(false);
  }
  function cancelHide() {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
  }

  return (
    <TooltipContext.Provider value={{ isOpen, show, scheduleHide, hideNow, cancelHide, anchorRef, panelRef, descriptionId }}>
      {children}
    </TooltipContext.Provider>
  );
}

export function TooltipTrigger({ className, children, ...props }) {
  const { isOpen, show, scheduleHide, hideNow, anchorRef, descriptionId } = useTooltipContext("TooltipTrigger");
  return (
    <span
      ref={anchorRef}
      data-slot="tooltip-trigger"
      className={cx("inline-flex", className)}
      onMouseEnter={show}
      onMouseLeave={scheduleHide}
      onFocus={show}
      onBlur={hideNow}
      aria-describedby={isOpen ? descriptionId : undefined}
      {...props}
    >
      {children}
    </span>
  );
}

export function TooltipContent({ className, children, side = "top", align = "center", sideOffset = 6, ...props }) {
  const { isOpen, anchorRef, panelRef, cancelHide, scheduleHide, descriptionId } = useTooltipContext("TooltipContent");
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, { side, align, offset: sideOffset });

  if (!isOpen) return null;

  const node = (
    <div
      ref={panelRef}
      id={descriptionId}
      role="tooltip"
      className="fixed z-50"
      style={{ top: `${top}px`, left: `${left}px` }}
      onMouseEnter={cancelHide}
      onMouseLeave={scheduleHide}
    >
      <div data-slot="tooltip-content" className={cx("w-fit rounded-md bg-foreground px-3 py-1.5 text-xs text-background", className)} {...props}>
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export default Tooltip;
