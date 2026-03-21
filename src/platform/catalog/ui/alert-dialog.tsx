import { cx } from "zeb";
import { useState } from "zeb";

interface AlertDialogProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

interface AlertDialogPartProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function AlertDialog({ open, defaultOpen = false, onOpenChange, children }: AlertDialogProps) {
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

  return <>{enhanced}</>;
}

export function AlertDialogTrigger({ children, _onOpen, asChild, className, ...rest }: any) {
  return (
    <button type="button" onClick={_onOpen} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function AlertDialogContent({ className, children, _isOpen, _onClose, ...rest }: any) {
  if (!_isOpen) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="fixed inset-0 bg-black/80" />
      <div
        role="alertdialog"
        aria-modal="true"
        className={cx(
          "relative z-50 grid w-full max-w-lg gap-4 border border-gray-200 bg-white p-6 shadow-lg rounded-xl",
          className
        )}
        {...rest}
      >
        {children}
      </div>
    </div>
  );
}

export function AlertDialogHeader({ className, children, ...rest }: AlertDialogPartProps) {
  return (
    <div className={cx("flex flex-col space-y-2 text-center sm:text-left", className)} {...rest}>
      {children}
    </div>
  );
}

export function AlertDialogFooter({ className, children, ...rest }: AlertDialogPartProps) {
  return (
    <div className={cx("flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2", className)} {...rest}>
      {children}
    </div>
  );
}

export function AlertDialogTitle({ className, children, ...rest }: AlertDialogPartProps) {
  return (
    <h2 className={cx("text-lg font-semibold", className)} {...rest}>
      {children}
    </h2>
  );
}

export function AlertDialogDescription({ className, children, ...rest }: AlertDialogPartProps) {
  return (
    <p className={cx("text-sm text-gray-500", className)} {...rest}>
      {children}
    </p>
  );
}

export function AlertDialogAction({ className, children, _onClose, onClick, ...rest }: any) {
  const handleClick = (e: any) => {
    onClick?.(e);
    _onClose?.();
  };
  return (
    <button
      type="button"
      onClick={handleClick}
      className={cx(
        "inline-flex items-center justify-center rounded-md bg-gray-900 px-4 py-2 text-sm font-medium text-white hover:bg-gray-800 focus:outline-none focus-visible:ring-1 focus-visible:ring-gray-950",
        className
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export function AlertDialogCancel({ className, children, _onClose, onClick, ...rest }: any) {
  const handleClick = (e: any) => {
    onClick?.(e);
    _onClose?.();
  };
  return (
    <button
      type="button"
      onClick={handleClick}
      className={cx(
        "inline-flex items-center justify-center rounded-md border border-gray-200 bg-white px-4 py-2 text-sm font-medium text-gray-900 hover:bg-gray-100 focus:outline-none focus-visible:ring-1 focus-visible:ring-gray-950",
        className
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export default AlertDialog;
