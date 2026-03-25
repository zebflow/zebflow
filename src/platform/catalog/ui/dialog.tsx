import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface DialogProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

interface DialogContentProps {
  className?: string;
  children?: any;
  // injected
  _isOpen?: boolean;
  _onClose?: () => void;
  [key: string]: any;
}

interface DialogPartProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Dialog({ open, defaultOpen = false, onOpenChange, children }: DialogProps) {
  const [internal, setInternal] = useState(defaultOpen);
  const controlled = open !== undefined;
  const isOpen = controlled ? open : internal;

  const toggle = (v: boolean) => {
    if (!controlled) setInternal(v);
    onOpenChange?.(v);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onOpen: () => toggle(true), _onClose: () => toggle(false) } };
  });

  return (
    <>
      {enhanced}
      <span hidden tw-variants="fixed inset-0 z-50 flex items-center justify-center bg-black/80 relative grid max-w-lg gap-4 border-gray-200 bg-white p-6 shadow-lg rounded-xl opacity-70 ring-offset-white transition-opacity hover:opacity-100 focus:ring-2 focus:ring-gray-950 focus:ring-offset-2 right-4 top-4 rounded-sm absolute space-y-1.5 flex-col sm:text-left flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 text-lg font-semibold leading-none tracking-tight text-sm text-gray-500" />
    </>
  );
}

export function DialogTrigger({ children, _onOpen, asChild, className, ...rest }: any) {
  if (asChild && children) {
    const child = Array.isArray(children) ? children[0] : children;
    if (child && typeof child === "object") {
      return { ...child, props: { ...child.props, onClick: _onOpen } };
    }
  }
  return (
    <button type="button" onClick={_onOpen} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function DialogContent({ className, children, _isOpen, _onClose, ...rest }: DialogContentProps) {
  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, () => _onClose?.());

  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/80" onClick={_onClose} />
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        className={cx(
          "relative z-50 grid w-full max-w-lg gap-4 border border-gray-200 bg-white p-6 shadow-lg rounded-xl",
          className
        )}
        {...rest}
      >
        {children}
        <button
          type="button"
          onClick={_onClose}
          className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-white transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-gray-950 focus:ring-offset-2"
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

export function DialogHeader({ className, children, ...rest }: DialogPartProps) {
  return (
    <div className={cx("flex flex-col space-y-1.5 text-center sm:text-left", className)} {...rest}>
      {children}
    </div>
  );
}

export function DialogFooter({ className, children, ...rest }: DialogPartProps) {
  return (
    <div className={cx("flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2", className)} {...rest}>
      {children}
    </div>
  );
}

export function DialogTitle({ className, children, ...rest }: DialogPartProps) {
  return (
    <h2 className={cx("text-lg font-semibold leading-none tracking-tight", className)} {...rest}>
      {children}
    </h2>
  );
}

export function DialogDescription({ className, children, ...rest }: DialogPartProps) {
  return (
    <p className={cx("text-sm text-gray-500", className)} {...rest}>
      {children}
    </p>
  );
}

export function DialogClose({ children, _onClose, asChild, className, ...rest }: any) {
  if (asChild && children) {
    const child = Array.isArray(children) ? children[0] : children;
    if (child && typeof child === "object") {
      return { ...child, props: { ...child.props, onClick: _onClose } };
    }
  }
  return (
    <button type="button" onClick={_onClose} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export default Dialog;
