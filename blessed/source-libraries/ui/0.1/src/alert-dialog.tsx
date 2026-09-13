import { cx, createContext, useContext, useRef, useId, useEffect, createPortal } from "zeb/react";
import { useControllable, useEscape, useFocusTrap, isVNode, composeEventHandlers } from "zeb/ui/hooks";
import { Button } from "zeb/ui/button";

/**
 * AlertDialog — shadcn's alert-dialog, on Zebflow's engine.
 *
 * Same context-based wiring as `zeb/ui/dialog` (no child-walking: every
 * sub-part reads `createContext`), the same trigger-wrapping rule for the
 * missing `asChild` (a plain-content trigger gets a real `<button>`; an
 * element child — usually `<Button>`/`<AlertDialogAction>` — gets wrapped in
 * a plain `inline-flex` span instead, so it is never nested inside another
 * button), and the same accessible-name wiring: `AlertDialogContent` carries
 * `role="alertdialog"`, `aria-modal`, `tabIndex={-1}`, and
 * `aria-labelledby`/`aria-describedby` pointed at `AlertDialogTitle`/
 * `AlertDialogDescription`'s `useId()`s. It portals to `document.body` in
 * the browser and renders inline during SSR, same as Dialog.
 *
 * The one behavioural difference from Dialog, matching upstream's intent
 * that an alert can't be dismissed by accident: Escape closes it, but a
 * click on the overlay does not — only `AlertDialogAction`/
 * `AlertDialogCancel` (both carry `data-dismiss`) do. Dropped versus
 * upstream: the `size="sm"` variant and the media/icon slot, both built on
 * `group-data-[...]`/`has-data-[...]` selectors the engine's Tailwind subset
 * does not compile.
 */

const AlertDialogContext = createContext(null);

function useAlertDialogContext(name) {
  const ctx = useContext(AlertDialogContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <AlertDialog>`);
  return ctx;
}

export function AlertDialog({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const titleId = useId();
  const descriptionId = useId();
  return <AlertDialogContext.Provider value={{ isOpen, setIsOpen, titleId, descriptionId }}>{children}</AlertDialogContext.Provider>;
}

export function AlertDialogTrigger({ className, children, onClick, ...props }) {
  const { setIsOpen } = useAlertDialogContext("AlertDialogTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(true));
  if (isVNode(children)) {
    return (
      <span data-slot="alert-dialog-trigger" className={cx("inline-flex", className)} onClick={handleClick} {...props}>
        {children}
      </span>
    );
  }
  return (
    <button type="button" data-slot="alert-dialog-trigger" className={cx("", className)} onClick={handleClick} {...props}>
      {children}
    </button>
  );
}

export function AlertDialogContent({ className, children, ...props }) {
  const { isOpen, setIsOpen, titleId, descriptionId } = useAlertDialogContext("AlertDialogContent");
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
    <div className="fixed inset-0 z-50 bg-foreground/50" data-slot="alert-dialog-overlay">
      <div
        ref={panelRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        tabIndex={-1}
        data-slot="alert-dialog-content"
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

export function AlertDialogHeader({ className, children, ...props }) {
  return (
    <div data-slot="alert-dialog-header" className={cx("flex flex-col gap-2 text-center sm:text-left", className)} {...props}>
      {children}
    </div>
  );
}

export function AlertDialogFooter({ className, children, ...props }) {
  return (
    <div data-slot="alert-dialog-footer" className={cx("flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)} {...props}>
      {children}
    </div>
  );
}

export function AlertDialogTitle({ className, children, id, ...props }) {
  const { titleId } = useAlertDialogContext("AlertDialogTitle");
  return (
    <h2 id={id ?? titleId} data-slot="alert-dialog-title" className={cx("text-lg font-semibold text-foreground", className)} {...props}>
      {children}
    </h2>
  );
}

export function AlertDialogDescription({ className, children, id, ...props }) {
  const { descriptionId } = useAlertDialogContext("AlertDialogDescription");
  return (
    <p id={id ?? descriptionId} data-slot="alert-dialog-description" className={cx("text-sm text-muted-foreground", className)} {...props}>
      {children}
    </p>
  );
}

export function AlertDialogAction({ className, onClick, children, ...props }) {
  const { setIsOpen } = useAlertDialogContext("AlertDialogAction");
  return (
    <Button data-slot="alert-dialog-action" data-dismiss="true" onClick={composeEventHandlers(onClick, () => setIsOpen(false))} className={className} {...props}>
      {children}
    </Button>
  );
}

export function AlertDialogCancel({ className, onClick, children, ...props }) {
  const { setIsOpen } = useAlertDialogContext("AlertDialogCancel");
  return (
    <Button
      variant="outline"
      data-slot="alert-dialog-cancel"
      data-dismiss="true"
      onClick={composeEventHandlers(onClick, () => setIsOpen(false))}
      className={className}
      {...props}
    >
      {children}
    </Button>
  );
}

export default AlertDialog;
