import { cx } from "zeb";
import { useState } from "zeb";

interface TooltipProviderProps {
  delayDuration?: number;
  children?: any;
  [key: string]: any;
}

interface TooltipProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

export function TooltipProvider({ delayDuration = 700, children }: TooltipProviderProps) {
  return <>{children}</>;
}

export function Tooltip({ open, defaultOpen = false, onOpenChange, children }: TooltipProps) {
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
    <span className="relative inline-flex">
      {enhanced}
    </span>
  );
}

export function TooltipTrigger({ children, _onOpen, _onClose, asChild, className, ...rest }: any) {
  if (asChild && children && typeof children === "object") {
    return { ...children, props: { ...children.props, onMouseEnter: _onOpen, onMouseLeave: _onClose, onFocus: _onOpen, onBlur: _onClose } };
  }
  return (
    <button
      type="button"
      onMouseEnter={_onOpen}
      onMouseLeave={_onClose}
      onFocus={_onOpen}
      onBlur={_onClose}
      className={cx("", className)}
      {...rest}
    >
      {children}
    </button>
  );
}

export function TooltipContent({ className, children, _isOpen, side = "top", sideOffset = 4, ...rest }: any) {
  if (!_isOpen) return null;

  const posClass = side === "bottom"
    ? `top-full mt-${sideOffset} left-1/2 -translate-x-1/2`
    : side === "left"
    ? `right-full top-1/2 -translate-y-1/2 mr-${sideOffset}`
    : side === "right"
    ? `left-full top-1/2 -translate-y-1/2 ml-${sideOffset}`
    : `bottom-full left-1/2 -translate-x-1/2`;

  const posStyle = side === "bottom"
    ? { top: "100%", marginTop: `${sideOffset}px`, left: "50%", transform: "translateX(-50%)" }
    : side === "left"
    ? { right: "100%", marginRight: `${sideOffset}px`, top: "50%", transform: "translateY(-50%)" }
    : side === "right"
    ? { left: "100%", marginLeft: `${sideOffset}px`, top: "50%", transform: "translateY(-50%)" }
    : { bottom: "100%", marginBottom: `${sideOffset}px`, left: "50%", transform: "translateX(-50%)" };

  return (
    <div
      role="tooltip"
      className={cx(
        "absolute z-50 overflow-hidden rounded-md bg-gray-900 px-3 py-1.5 text-xs text-white animate-in fade-in-0 zoom-in-95",
        className
      )}
      style={posStyle}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Tooltip;
