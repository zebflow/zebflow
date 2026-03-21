import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface DropdownMenuProps {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  children?: any;
  [key: string]: any;
}

export function DropdownMenu({ open, defaultOpen = false, onOpenChange, children }: DropdownMenuProps) {
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
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onToggle: () => toggle(!isOpen), _onClose: () => toggle(false) } };
  });

  return (
    <div ref={containerRef} className="relative inline-block">
      {enhanced}
    </div>
  );
}

export function DropdownMenuTrigger({ children, _onToggle, asChild, className, ...rest }: any) {
  if (asChild && children && typeof children === "object") {
    return { ...children, props: { ...children.props, onClick: _onToggle } };
  }
  return (
    <button type="button" onClick={_onToggle} className={cx("", className)} {...rest}>
      {children}
    </button>
  );
}

export function DropdownMenuContent({ className, children, _isOpen, _onClose, align = "start", sideOffset = 4, ...rest }: any) {
  if (!_isOpen) return null;

  const alignClass = align === "end" ? "right-0" : "left-0";

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _onClose } };
  });

  return (
    <div
      role="menu"
      className={cx(
        `absolute z-50 min-w-[8rem] overflow-hidden rounded-md border border-gray-200 bg-white p-1 shadow-md`,
        alignClass,
        className
      )}
      style={{ top: "100%", marginTop: `${sideOffset}px` }}
      {...rest}
    >
      {enhanced}
    </div>
  );
}

export function DropdownMenuItem({ className, children, _onClose, onClick, inset, disabled, ...rest }: any) {
  const handleClick = (e: any) => {
    if (disabled) return;
    onClick?.(e);
    _onClose?.();
  };
  return (
    <div
      role="menuitem"
      onClick={handleClick}
      className={cx(
        "relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-colors",
        disabled ? "pointer-events-none opacity-50" : "hover:bg-gray-100 hover:text-gray-900 cursor-pointer",
        inset ? "pl-8" : "",
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function DropdownMenuSeparator({ className, ...rest }: any) {
  return (
    <div className={cx("-mx-1 my-1 h-px bg-gray-100", className)} role="separator" {...rest} />
  );
}

export function DropdownMenuLabel({ className, children, inset, ...rest }: any) {
  return (
    <div
      className={cx(
        "px-2 py-1.5 text-sm font-semibold",
        inset ? "pl-8" : "",
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function DropdownMenuCheckboxItem({ className, children, checked, onCheckedChange, _onClose, ...rest }: any) {
  return (
    <div
      role="menuitemcheckbox"
      aria-checked={checked}
      onClick={() => { onCheckedChange?.(!checked); }}
      className={cx(
        "relative flex cursor-pointer select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none transition-colors hover:bg-gray-100 hover:text-gray-900",
        className
      )}
      {...rest}
    >
      <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        {checked && (
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        )}
      </span>
      {children}
    </div>
  );
}

export function DropdownMenuRadioGroup({ className, children, value, onValueChange, ...rest }: any) {
  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _groupValue: value, _onValueChange: onValueChange } };
  });
  return (
    <div role="group" className={cx("", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function DropdownMenuRadioItem({ className, children, value, _groupValue, _onValueChange, _onClose, ...rest }: any) {
  const checked = _groupValue === value;
  return (
    <div
      role="menuitemradio"
      aria-checked={checked}
      onClick={() => { _onValueChange?.(value); _onClose?.(); }}
      className={cx(
        "relative flex cursor-pointer select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none transition-colors hover:bg-gray-100 hover:text-gray-900",
        className
      )}
      {...rest}
    >
      <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        {checked && (
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <circle cx="12" cy="12" r="4" fill="currentColor" />
          </svg>
        )}
      </span>
      {children}
    </div>
  );
}

export default DropdownMenu;
