import { useRef, useEffect, cx } from "zeb";
import Button from "@/components/ui/button";

/**
 * Dialog — controlled native <dialog> modal.
 *
 * Critical design rule: `display: flex` lives on the INNER wrapper div,
 * NOT on the <dialog> element itself.
 *
 * Why: The native <dialog> UA stylesheet hides closed dialogs via:
 *   dialog:not([open]) { display: none }   ← UA cascade level
 * Any Tailwind class like `flex` is Author-level and unconditionally beats UA.
 * Putting `flex` on <dialog> makes it permanently visible regardless of open state.
 * Keeping flex on an inner <div> leaves the <dialog> element's display entirely
 * to the UA, so open/close works correctly through showModal() / close().
 *
 * Props:
 *   open      boolean  — controlled open state; drives showModal() / close() internally
 *   onClose   fn       — called on Escape key, X button, or backdrop click
 *   title     string   — header title; omit prop to suppress the header row entirely
 *   wClass    string   — Tailwind width class applied to the <dialog> element (default "w-[22.5rem]")
 *   footer    JSX      — footer slot, rendered in a flex row (Save / Cancel buttons etc.)
 *   className string   — extra classes merged onto the scrollable body div
 *   children  JSX      — body content
 */
export default function Dialog({ open, onClose, title, wClass, footer, className, children }) {
  const ref = useRef(null as HTMLDialogElement | null);

  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    else if (!open && d.open) d.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      onClose={onClose}
      onClick={(e) => { if (e.target === e.currentTarget) onClose?.(); }}
      className={cx("p-0 m-auto inset-0 w-full border border-[var(--studio-border)] rounded-lg bg-[var(--studio-panel)] text-[var(--studio-text)] shadow-[0_12px_32px_rgba(0,0,0,0.4)] overflow-hidden max-h-[85vh]", wClass ?? "max-w-sm")}
    >
      {/* flex layout lives here — NOT on <dialog> so UA display:none wins for closed state */}
      <div className="flex flex-col max-h-[85vh]">
        {title != null && (
          <div className="flex shrink-0 items-center justify-between px-[0.85rem] py-[0.6rem] border-b border-[var(--studio-border)] text-[0.78rem] font-semibold text-[var(--studio-text)]">
            <span>{title}</span>
            <Button variant="ghost" size="icon" onClick={onClose} className="size-6 text-[var(--studio-text-soft)]">✕</Button>
          </div>
        )}
        <div className={cx("flex flex-col flex-1 min-h-0 overflow-y-auto p-[0.85rem] gap-[0.6rem]", className)}>
          {children}
        </div>
        {footer && (
          <div className="flex shrink-0 items-center justify-end gap-2 px-[0.85rem] py-[0.6rem] border-t border-[var(--studio-border)]">
            {footer}
          </div>
        )}
      </div>
    </dialog>
  );
}
