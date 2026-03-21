import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface PopoverProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

export function Popover({ open, defaultOpen = false, onOpenChange, children }: PopoverProps) {
  const [internal, setInternal] = useState(defaultOpen);
  const controlled = open !== undefined;
  const isOpen = controlled ? open : internal;

  const toggle = (v: boolean) => {
    if (!controlled) setInternal(v);
    onOpenChange?.(v);
  };

  const containerRef = useRef<HTMLDivElement>(null);
  useClickAway(containerRef, () => toggle(false));

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onOpen: () => toggle(true), _onClose: () => toggle(false) } };
  });

  return (
    <div ref={containerRef} className="relative inline-block">
      {enhanced}
    </div>
  );
}

export function PopoverTrigger({ children, _onOpen, asChild, className, ...rest }: any) {
  const handleClick = () => _onOpen?.();
  if (asChild && children && typeof children === "object") {
    return { ...children, props: { ...children.props, onClick: handleClick } };
  }
  return (
    <button type="button" onClick={handleClick} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function PopoverContent({ className, children, _isOpen, align = "center", sideOffset = 4, ...rest }: any) {
  if (!_isOpen) return null;

  const alignClass = align === "start" ? "left-0" : align === "end" ? "right-0" : "left-1/2 -translate-x-1/2";

  return (
    <div
      role="dialog"
      className={cx(
        `absolute z-50 w-72 rounded-md border border-gray-200 bg-white p-4 shadow-md outline-none top-full mt-${sideOffset}`,
        alignClass,
        className
      )}
      style={{ marginTop: `${sideOffset}px` }}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Popover;
