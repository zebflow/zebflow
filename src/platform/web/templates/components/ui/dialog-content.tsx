import { cx } from "zeb";

export default function DialogContent({ className, children, _isOpen, _onClose, ...rest }: any) {
  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="fixed inset-0 bg-black/80" onClick={_onClose} />
      <div
        role="dialog"
        aria-modal="true"
        className={cx(
          "relative z-50 w-full max-w-lg",
          "bg-ui-bg text-ui-text border border-ui-border",
          "shadow-xl rounded-xl",
          "flex flex-col",
          "max-h-[85vh] overflow-y-auto",
          className
        )}
        {...rest}
      >
        {children}
        <button
          type="button"
          onClick={_onClose}
          className="absolute right-4 top-4 z-10 rounded-sm text-ui-text-muted opacity-70 hover:opacity-100 transition-opacity"
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
