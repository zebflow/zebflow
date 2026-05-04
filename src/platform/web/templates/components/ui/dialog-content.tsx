import { cx } from "zeb";

export default function DialogContent({ className, children, _isOpen, _onClose, style, ...rest }: any) {
  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="fixed inset-0 bg-[rgba(2,6,23,0.62)] backdrop-blur-[1px]" onClick={_onClose} />
      <div
        role="dialog"
        aria-modal="true"
        className={cx(
          "relative z-50 w-full max-w-lg",
          "border border-border bg-[var(--color-surface,#161616)] text-body",
          "rounded-[0.65rem] shadow-[0_24px_55px_rgba(2,6,23,0.32)]",
          "flex flex-col",
          "max-h-[calc(100vh-2rem)] overflow-y-auto",
          className
        )}
        style={{
          ...(style || {}),
          backgroundColor: style?.backgroundColor || "var(--color-surface, #161616)",
          opacity: 1,
        }}
        {...rest}
      >
        {children}
        <button
          type="button"
          onClick={_onClose}
          className="absolute right-4 top-4 z-10 rounded-sm text-body-soft opacity-70 transition-opacity hover:opacity-100 hover:text-body"
          aria-label="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 6 6 18" /><path d="m6 6 12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}
