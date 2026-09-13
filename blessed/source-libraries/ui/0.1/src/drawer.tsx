import { cx, createContext, useContext, useRef, useId, useEffect, createPortal } from "zeb/react";
import { useControllable, useEscape, useFocusTrap, isVNode, composeEventHandlers } from "zeb/ui/hooks";

/**
 * Drawer — shadcn's drawer (built on vaul upstream) reduced to what the
 * engine can give it: a bottom sheet with a grab handle, no drag-to-dismiss
 * gesture. Wiring matches `zeb/ui/sheet`: `Drawer`/`DrawerTrigger`/
 * `DrawerContent`/… compose through `createContext`, `DrawerContent` owns
 * `role="dialog"`, `aria-modal`, `tabIndex={-1}` and
 * `aria-labelledby`/`aria-describedby` wired to `DrawerTitle`/
 * `DrawerDescription`'s `useId()`s, and portals to `document.body` in the
 * browser (rendering inline during SSR). Escape and an overlay click both
 * close it; `DrawerTrigger` wraps an element child in a plain `inline-flex`
 * span rather than its own `<button>`, so there is no button-in-button.
 * Dropped versus upstream: the top/left/right directions and the swipe
 * gesture — vaul's physics do not have an engine equivalent; a click-driven
 * bottom sheet covers the common case.
 */

const DrawerContext = createContext(null);

function useDrawerContext(name) {
  const ctx = useContext(DrawerContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Drawer>`);
  return ctx;
}

export function Drawer({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const titleId = useId();
  const descriptionId = useId();
  return <DrawerContext.Provider value={{ isOpen, setIsOpen, titleId, descriptionId }}>{children}</DrawerContext.Provider>;
}

export function DrawerTrigger({ className, children, onClick, ...props }) {
  const { setIsOpen } = useDrawerContext("DrawerTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(true));
  if (isVNode(children)) {
    return (
      <span data-slot="drawer-trigger" className={cx("inline-flex", className)} onClick={handleClick} {...props}>
        {children}
      </span>
    );
  }
  return (
    <button type="button" data-slot="drawer-trigger" className={cx("", className)} onClick={handleClick} {...props}>
      {children}
    </button>
  );
}

export function DrawerContent({ className, children, ...props }) {
  const { isOpen, setIsOpen, titleId, descriptionId } = useDrawerContext("DrawerContent");
  const panelRef = useRef(null);

  useEscape(isOpen, () => setIsOpen(false));
  useFocusTrap(panelRef, isOpen);

  useEffect(() => {
    if (!isOpen) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = previous;
    };
  }, [isOpen]);

  if (!isOpen) return null;

  function onPanelClick(event) {
    event.stopPropagation();
    const target = event.target;
    if (target && target.closest && target.closest('[data-dismiss="true"]')) {
      setIsOpen(false);
    }
  }

  const node = (
    <div className="fixed inset-0 z-50 bg-foreground/50" data-slot="drawer-overlay" onClick={() => setIsOpen(false)}>
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        tabIndex={-1}
        data-slot="drawer-content"
        onClick={onPanelClick}
        className={cx(
          "fixed inset-x-0 bottom-0 z-50 flex max-h-[80vh] flex-col rounded-t-lg border-t border-border bg-background",
          className
        )}
        {...props}
      >
        <div className="mx-auto mt-4 h-2 w-24 shrink-0 rounded-full bg-muted" />
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function DrawerHeader({ className, children, ...props }) {
  return (
    <div data-slot="drawer-header" className={cx("flex flex-col gap-1.5 p-4 text-center", className)} {...props}>
      {children}
    </div>
  );
}

export function DrawerFooter({ className, children, ...props }) {
  return (
    <div data-slot="drawer-footer" className={cx("mt-auto flex flex-col gap-2 p-4", className)} {...props}>
      {children}
    </div>
  );
}

export function DrawerTitle({ className, children, id, ...props }) {
  const { titleId } = useDrawerContext("DrawerTitle");
  return (
    <h2 id={id ?? titleId} data-slot="drawer-title" className={cx("font-semibold text-foreground", className)} {...props}>
      {children}
    </h2>
  );
}

export function DrawerDescription({ className, children, id, ...props }) {
  const { descriptionId } = useDrawerContext("DrawerDescription");
  return (
    <p id={id ?? descriptionId} data-slot="drawer-description" className={cx("text-sm text-muted-foreground", className)} {...props}>
      {children}
    </p>
  );
}

export function DrawerClose({ className, children, onClick, ...props }) {
  const { setIsOpen } = useDrawerContext("DrawerClose");
  return (
    <button
      type="button"
      data-slot="drawer-close"
      data-dismiss="true"
      className={cx("", className)}
      onClick={composeEventHandlers(onClick, () => setIsOpen(false))}
      {...props}
    >
      {children}
    </button>
  );
}

export default Drawer;
