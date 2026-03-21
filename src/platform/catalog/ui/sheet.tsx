import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface SheetProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

const SIDE_CLASSES = {
  top:    "inset-x-0 top-0 border-b rounded-b-xl",
  bottom: "inset-x-0 bottom-0 border-t rounded-t-xl",
  left:   "inset-y-0 left-0 h-full w-3/4 border-r rounded-r-xl sm:max-w-sm",
  right:  "inset-y-0 right-0 h-full w-3/4 border-l rounded-l-xl sm:max-w-sm",
};

export function Sheet({ open, defaultOpen = false, onOpenChange, children }: SheetProps) {
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

export function SheetTrigger({ children, _onOpen, asChild, className, ...rest }: any) {
  return (
    <button type="button" onClick={_onOpen} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function SheetContent({ side = "right", className, children, _isOpen, _onClose, ...rest }: any) {
  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, () => _onClose?.());

  if (!_isOpen) return null;

  return (
    <div className="fixed inset-0 z-50">
      <div className="fixed inset-0 bg-black/80" onClick={_onClose} />
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        className={cx(
          "fixed z-50 gap-4 bg-white p-6 shadow-lg border-gray-200",
          SIDE_CLASSES[side as keyof typeof SIDE_CLASSES] ?? SIDE_CLASSES.right,
          className
        )}
        {...rest}
      >
        {children}
        <button
          type="button"
          onClick={_onClose}
          className="absolute right-4 top-4 rounded-sm opacity-70 hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-gray-950"
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

export function SheetHeader({ className, children, ...rest }: any) {
  return (
    <div className={cx("flex flex-col space-y-2 text-center sm:text-left", className)} {...rest}>
      {children}
    </div>
  );
}

export function SheetFooter({ className, children, ...rest }: any) {
  return (
    <div className={cx("flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2", className)} {...rest}>
      {children}
    </div>
  );
}

export function SheetTitle({ className, children, ...rest }: any) {
  return (
    <h2 className={cx("text-lg font-semibold text-gray-900", className)} {...rest}>
      {children}
    </h2>
  );
}

export function SheetDescription({ className, children, ...rest }: any) {
  return (
    <p className={cx("text-sm text-gray-500", className)} {...rest}>
      {children}
    </p>
  );
}

export function SheetClose({ children, _onClose, className, ...rest }: any) {
  return (
    <button type="button" onClick={_onClose} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export default Sheet;
