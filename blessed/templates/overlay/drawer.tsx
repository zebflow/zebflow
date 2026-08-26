import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface DrawerProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

export function Drawer({ open, defaultOpen = false, onOpenChange, children }: DrawerProps) {
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

export function DrawerTrigger({ children, _onOpen, className, ...rest }: any) {
  return (
    <button type="button" onClick={_onOpen} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function DrawerContent({ className, children, _isOpen, _onClose, ...rest }: any) {
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
          "fixed inset-x-0 bottom-0 z-50 mt-24 flex h-auto flex-col rounded-t-xl border border-gray-200 bg-white",
          className
        )}
        {...rest}
      >
        <div className="mx-auto mt-4 h-1.5 w-12 rounded-full bg-gray-200" />
        {children}
      </div>
    </div>
  );
}

export function DrawerHeader({ className, children, ...rest }: any) {
  return (
    <div className={cx("grid gap-1.5 p-4 text-center sm:text-left", className)} {...rest}>
      {children}
    </div>
  );
}

export function DrawerFooter({ className, children, ...rest }: any) {
  return (
    <div className={cx("mt-auto flex flex-col gap-2 p-4", className)} {...rest}>
      {children}
    </div>
  );
}

export function DrawerTitle({ className, children, ...rest }: any) {
  return (
    <h2 className={cx("text-lg font-semibold leading-none tracking-tight", className)} {...rest}>
      {children}
    </h2>
  );
}

export function DrawerDescription({ className, children, ...rest }: any) {
  return (
    <p className={cx("text-sm text-gray-500", className)} {...rest}>
      {children}
    </p>
  );
}

export function DrawerClose({ children, _onClose, className, ...rest }: any) {
  return (
    <button type="button" onClick={_onClose} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export default Drawer;
