import { cx } from "zeb";
import { useState } from "zeb";

interface CollapsibleProps {
  defaultOpen?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface CollapsibleTriggerProps {
  className?: string;
  children?: any;
  asChild?: boolean;
  // injected
  _isOpen?: boolean;
  _onToggle?: () => void;
  [key: string]: any;
}

interface CollapsibleContentProps {
  className?: string;
  children?: any;
  // injected
  _isOpen?: boolean;
  [key: string]: any;
}

export function Collapsible({ defaultOpen = false, open, onOpenChange, className, children, ...rest }: CollapsibleProps) {
  const [internal, setInternal] = useState(defaultOpen);
  const controlled = open !== undefined;
  const isOpen = controlled ? open : internal;

  const toggle = () => {
    const next = !isOpen;
    if (!controlled) setInternal(next);
    onOpenChange?.(next);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onToggle: toggle } };
  });

  return (
    <div className={cx("", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function CollapsibleTrigger({ className, children, _isOpen, _onToggle, asChild, ...rest }: CollapsibleTriggerProps) {
  return (
    <button
      type="button"
      onClick={_onToggle}
      aria-expanded={_isOpen}
      className={cx("", className)}
      {...rest}
    >
      {children}
    </button>
  );
}

export function CollapsibleContent({ className, children, _isOpen, ...rest }: CollapsibleContentProps) {
  if (!_isOpen) return null;
  return (
    <div className={cx("", className)} {...rest}>
      {children}
    </div>
  );
}

export default Collapsible;
