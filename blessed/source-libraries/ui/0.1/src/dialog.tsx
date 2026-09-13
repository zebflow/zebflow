import { cx, createContext, useContext, useRef, useId, useEffect, createPortal } from "zeb/react";
import { useControllable, useEscape, useFocusTrap, isVNode, composeEventHandlers } from "zeb/ui/hooks";

/**
 * Dialog — shadcn's dialog, on Zebflow's engine.
 *
 * `Dialog`/`DialogTrigger`/`DialogContent`/… compose through `createContext`
 * exactly as upstream's Radix primitive does, so a shadcn snippet's nesting
 * works unchanged: no child-walking, no `child.type === X` detection.
 * `DialogContent` owns the overlay, the panel, `role="dialog"`, `aria-modal`,
 * and `aria-labelledby`/`aria-describedby` wired to `DialogTitle`/
 * `DialogDescription` via `useId()`; it renders through `createPortal` to
 * `document.body` in the browser (there is no `document` during SSR, so it
 * renders inline there instead — the only place that check has to live).
 * Escape and a click on the overlay close it; a click inside the panel does
 * not, though any click that bubbles through an element carrying
 * `data-dismiss` (`DialogClose` sets it, and calls the context directly)
 * still does. Focus is trapped in the panel while open and restored on
 * close; the body's scroll is locked while open.
 *
 * There is no `asChild`/Slot on this engine: `DialogTrigger` renders its own
 * `<button>` for plain content, but when it is given an element — typically
 * a `<Button>` — it wraps that element in a plain `inline-flex` span instead,
 * so a shadcn snippet's usual `<DialogTrigger asChild><Button>…` never nests
 * a button inside a button.
 */

const DialogContext = createContext(null);

function useDialogContext(name) {
  const ctx = useContext(DialogContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Dialog>`);
  return ctx;
}

export function Dialog({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const titleId = useId();
  const descriptionId = useId();
  return <DialogContext.Provider value={{ isOpen, setIsOpen, titleId, descriptionId }}>{children}</DialogContext.Provider>;
}

export function DialogTrigger({ className, children, onClick, ...props }) {
  const { setIsOpen } = useDialogContext("DialogTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(true));
  if (isVNode(children)) {
    return (
      <span data-slot="dialog-trigger" className={cx("inline-flex", className)} onClick={handleClick} {...props}>
        {children}
      </span>
    );
  }
  return (
    <button type="button" data-slot="dialog-trigger" className={cx("", className)} onClick={handleClick} {...props}>
      {children}
    </button>
  );
}

export function DialogContent({ className, children, ...props }) {
  const { isOpen, setIsOpen, titleId, descriptionId } = useDialogContext("DialogContent");
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
    <div className="fixed inset-0 z-50 bg-foreground/50" data-slot="dialog-overlay" onClick={() => setIsOpen(false)}>
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        tabIndex={-1}
        data-slot="dialog-content"
        onClick={onPanelClick}
        className={cx(
          "fixed top-1/2 left-1/2 z-50 grid w-full max-w-lg -translate-x-1/2 -translate-y-1/2 gap-4 rounded-lg border border-border bg-background p-6 shadow-lg outline-none",
          className
        )}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function DialogHeader({ className, children, ...props }) {
  return (
    <div data-slot="dialog-header" className={cx("flex flex-col gap-2 text-center sm:text-left", className)} {...props}>
      {children}
    </div>
  );
}

export function DialogFooter({ className, children, ...props }) {
  return (
    <div data-slot="dialog-footer" className={cx("flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)} {...props}>
      {children}
    </div>
  );
}

export function DialogTitle({ className, children, id, ...props }) {
  const { titleId } = useDialogContext("DialogTitle");
  return (
    <h2 id={id ?? titleId} data-slot="dialog-title" className={cx("text-lg leading-none font-semibold text-foreground", className)} {...props}>
      {children}
    </h2>
  );
}

export function DialogDescription({ className, children, id, ...props }) {
  const { descriptionId } = useDialogContext("DialogDescription");
  return (
    <p id={id ?? descriptionId} data-slot="dialog-description" className={cx("text-sm text-muted-foreground", className)} {...props}>
      {children}
    </p>
  );
}

export function DialogClose({ className, children, onClick, ...props }) {
  const { setIsOpen } = useDialogContext("DialogClose");
  return (
    <button
      type="button"
      data-slot="dialog-close"
      data-dismiss="true"
      className={cx("", className)}
      onClick={composeEventHandlers(onClick, () => setIsOpen(false))}
      {...props}
    >
      {children}
    </button>
  );
}

export default Dialog;
