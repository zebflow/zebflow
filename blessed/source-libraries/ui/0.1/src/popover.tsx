import { cx, createContext, useContext, useRef, useEffect, createPortal } from "zeb/react";
import { useControllable, useClickAway, useEscape, useAnchoredPosition, isVNode, composeEventHandlers } from "zeb/ui/hooks";

/**
 * Popover — shadcn's popover, on Zebflow's engine.
 *
 * `Popover`/`PopoverTrigger`/`PopoverContent` compose through `createContext`
 * (no child-walking). `PopoverTrigger` anchors the panel: it renders its own
 * `<button>` for plain content, or wraps an element child (usually
 * `<Button>`) in an `inline-flex` span instead, so there is no
 * button-in-button — either way the rendered node is what `PopoverContent`
 * positions against. `PopoverContent` reads its own `side`/`align`/
 * `sideOffset` (falling back to `bottom`/`center`/`8`, upstream's defaults)
 * instead of them being ignored, and portals to `document.body` in the
 * browser (rendering inline, in place, during SSR). A click outside the
 * trigger+panel pair, focus moving outside both, or Escape all close it —
 * click-away is gated on `isOpen` and checks both the trigger and the
 * (portaled) panel, since a portaled panel is no longer a DOM descendant of
 * the trigger's tree.
 */

const PopoverContext = createContext(null);

function usePopoverContext(name) {
  const ctx = useContext(PopoverContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Popover>`);
  return ctx;
}

export function Popover({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const anchorRef = useRef(null);
  const panelRef = useRef(null);

  useClickAway([anchorRef, panelRef], () => setIsOpen(false), isOpen);
  useEscape(isOpen, () => setIsOpen(false));

  useEffect(() => {
    if (!isOpen) return;
    function onFocusIn(event) {
      const inTrigger = anchorRef.current && anchorRef.current.contains(event.target);
      const inPanel = panelRef.current && panelRef.current.contains(event.target);
      if (!inTrigger && !inPanel) setIsOpen(false);
    }
    document.addEventListener("focusin", onFocusIn, true);
    return () => document.removeEventListener("focusin", onFocusIn, true);
  }, [isOpen]);

  return <PopoverContext.Provider value={{ isOpen, setIsOpen, anchorRef, panelRef }}>{children}</PopoverContext.Provider>;
}

export function PopoverTrigger({ className, children, onClick, ...props }) {
  const { isOpen, setIsOpen, anchorRef } = usePopoverContext("PopoverTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(!isOpen));
  if (isVNode(children)) {
    return (
      <span ref={anchorRef} data-slot="popover-trigger" className={cx("inline-flex", className)} onClick={handleClick} {...props}>
        {children}
      </span>
    );
  }
  return (
    <button ref={anchorRef} type="button" data-slot="popover-trigger" className={cx("", className)} onClick={handleClick} {...props}>
      {children}
    </button>
  );
}

export function PopoverContent({ className, children, side = "bottom", align = "center", sideOffset = 8, ...props }) {
  const { isOpen, anchorRef, panelRef } = usePopoverContext("PopoverContent");
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, { side, align, offset: sideOffset });

  if (!isOpen) return null;

  const node = (
    <div ref={panelRef} className="fixed z-50" style={{ top: `${top}px`, left: `${left}px` }}>
      <div
        tabIndex={-1}
        data-slot="popover-content"
        className={cx("w-72 rounded-md border border-border bg-popover p-4 text-popover-foreground shadow-md outline-none", className)}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export default Popover;
