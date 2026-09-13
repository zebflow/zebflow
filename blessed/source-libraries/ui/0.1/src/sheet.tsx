import { cx, createContext, useContext, useRef, useId, useEffect, createPortal } from "zeb/react";
import { useControllable, useEscape, useFocusTrap, isVNode, composeEventHandlers } from "zeb/ui/hooks";

/**
 * Sheet — shadcn's sheet (a dialog that slides in from an edge), on
 * Zebflow's engine. Wiring matches `zeb/ui/dialog`: `Sheet`/`SheetTrigger`/
 * `SheetContent`/… compose through `createContext`, `SheetContent` owns
 * `role="dialog"`, `aria-modal`, `tabIndex={-1}` and
 * `aria-labelledby`/`aria-describedby` wired to `SheetTitle`/
 * `SheetDescription`'s `useId()`s, and portals to `document.body` in the
 * browser (rendering inline during SSR, where there is no `document`).
 * Escape and an overlay click both close it. `SheetTrigger` wraps an
 * element child (usually `<Button>`) in a plain `inline-flex` span instead
 * of its own `<button>`, so there is no `asChild`-shaped button-in-button.
 * `side` lives on `SheetContent`, not on `Sheet`, since that is where
 * upstream puts it.
 */

const SIDE = {
  top: "inset-x-0 top-0 h-auto border-b border-border",
  right: "inset-y-0 right-0 h-full w-3/4 border-l border-border sm:max-w-sm",
  bottom: "inset-x-0 bottom-0 h-auto border-t border-border",
  left: "inset-y-0 left-0 h-full w-3/4 border-r border-border sm:max-w-sm",
};

const SheetContext = createContext(null);

function useSheetContext(name) {
  const ctx = useContext(SheetContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Sheet>`);
  return ctx;
}

export function Sheet({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const titleId = useId();
  const descriptionId = useId();
  return <SheetContext.Provider value={{ isOpen, setIsOpen, titleId, descriptionId }}>{children}</SheetContext.Provider>;
}

export function SheetTrigger({ className, children, onClick, ...props }) {
  const { setIsOpen } = useSheetContext("SheetTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(true));
  if (isVNode(children)) {
    return (
      <span data-slot="sheet-trigger" className={cx("inline-flex", className)} onClick={handleClick} {...props}>
        {children}
      </span>
    );
  }
  return (
    <button type="button" data-slot="sheet-trigger" className={cx("", className)} onClick={handleClick} {...props}>
      {children}
    </button>
  );
}

export function SheetContent({ className, children, side = "right", ...props }) {
  const { isOpen, setIsOpen, titleId, descriptionId } = useSheetContext("SheetContent");
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
    <div className="fixed inset-0 z-50 bg-foreground/50" data-slot="sheet-overlay" onClick={() => setIsOpen(false)}>
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        tabIndex={-1}
        data-slot="sheet-content"
        onClick={onPanelClick}
        className={cx("fixed z-50 flex flex-col gap-4 bg-background shadow-lg", SIDE[side] ?? SIDE.right, className)}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function SheetHeader({ className, children, ...props }) {
  return (
    <div data-slot="sheet-header" className={cx("flex flex-col gap-1.5 p-4", className)} {...props}>
      {children}
    </div>
  );
}

export function SheetFooter({ className, children, ...props }) {
  return (
    <div data-slot="sheet-footer" className={cx("mt-auto flex flex-col gap-2 p-4", className)} {...props}>
      {children}
    </div>
  );
}

export function SheetTitle({ className, children, id, ...props }) {
  const { titleId } = useSheetContext("SheetTitle");
  return (
    <h2 id={id ?? titleId} data-slot="sheet-title" className={cx("font-semibold text-foreground", className)} {...props}>
      {children}
    </h2>
  );
}

export function SheetDescription({ className, children, id, ...props }) {
  const { descriptionId } = useSheetContext("SheetDescription");
  return (
    <p id={id ?? descriptionId} data-slot="sheet-description" className={cx("text-sm text-muted-foreground", className)} {...props}>
      {children}
    </p>
  );
}

export function SheetClose({ className, children, onClick, ...props }) {
  const { setIsOpen } = useSheetContext("SheetClose");
  return (
    <button
      type="button"
      data-slot="sheet-close"
      data-dismiss="true"
      className={cx("", className)}
      onClick={composeEventHandlers(onClick, () => setIsOpen(false))}
      {...props}
    >
      {children}
    </button>
  );
}

export default Sheet;
