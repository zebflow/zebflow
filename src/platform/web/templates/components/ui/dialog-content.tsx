import { cx } from "zeb";

export default function DialogContent({ className, children, _isOpen, _onClose, ...rest }: any) {
  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/80" onClick={_onClose} />
      <div
        role="dialog"
        aria-modal="true"
        className={cx(
          "relative z-50 w-full max-w-lg border border-border bg-surface text-body p-6 shadow-lg rounded-xl flex flex-col gap-4",
          className
        )}
        {...rest}
      >
        {children}
        <button
          type="button"
          onClick={_onClose}
          className="absolute right-4 top-4 opacity-60 hover:opacity-100 transition-opacity text-body-soft"
          aria-label="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <path d="M18 6 6 18" /><path d="m6 6 12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}
