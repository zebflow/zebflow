import { cx, useEffect } from "zeb/react";

function dialogSizeClass(size: string | undefined, className: any) {
  const classText = typeof className === "string" ? className : "";
  if (!size && classText.includes("max-w-")) return "";
  switch (size || "md") {
    case "sm":
      return "max-w-sm";
    case "lg":
      return "max-w-xl";
    case "xl":
      return "max-w-2xl";
    case "wide":
      return "max-w-5xl";
    case "full":
      return "max-w-7xl";
    case "md":
    default:
      return "max-w-lg";
  }
}

export default function DialogContent({ className, children, _isOpen, _onClose, style, size, ...rest }: any) {
  useEffect(() => {
    if (!_isOpen) return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      _onClose?.();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [_isOpen, _onClose]);

  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto overscroll-contain px-4 py-6 sm:py-8">
      <div className="fixed inset-0 bg-[rgba(2,6,23,0.62)] backdrop-blur-[1px]" onClick={_onClose} />
      <div className="relative z-50 flex min-h-full items-center justify-center">
        <div
          role="dialog"
          aria-modal="true"
          className={cx(
            "relative z-50 w-full",
            dialogSizeClass(size, className),
            "border border-[var(--color-border,var(--color-ui-border))] bg-[var(--color-surface,var(--color-ui-bg))] text-[var(--color-body,var(--color-ui-text))]",
            "rounded-[0.65rem] shadow-[0_24px_55px_rgba(2,6,23,0.32)]",
            "flex min-h-0 flex-col",
            "overflow-y-auto overscroll-contain",
            className
          )}
          style={{
            ...(style || {}),
            maxHeight: style?.maxHeight || "calc(100dvh - 3rem)",
            overflowY: style?.overflowY || "auto",
            backgroundColor: style?.backgroundColor || "var(--color-surface, var(--color-ui-bg))",
            opacity: 1,
          }}
          {...rest}
        >
          {children}
          <button
            type="button"
            onClick={_onClose}
            className="absolute right-4 top-4 z-10 rounded-sm text-[var(--color-body-soft,var(--color-ui-text-soft))] opacity-70 transition-opacity hover:opacity-100 hover:text-[var(--color-body,var(--color-ui-text))]"
            aria-label="Close"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M18 6 6 18" /><path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
