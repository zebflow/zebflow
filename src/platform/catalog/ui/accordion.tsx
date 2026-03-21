import { cx } from "zeb";
import { useState } from "zeb";

interface AccordionProps {
  type?: "single" | "multiple";
  defaultValue?: string | string[];
  value?: string | string[];
  onValueChange?: (value: string | string[]) => void;
  collapsible?: boolean;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface AccordionItemProps {
  value: string;
  className?: string;
  children?: any;
  // injected
  _openItems?: string[];
  _onToggle?: (v: string) => void;
  [key: string]: any;
}

interface AccordionTriggerProps {
  className?: string;
  children?: any;
  // injected
  _isOpen?: boolean;
  _onToggle?: () => void;
  [key: string]: any;
}

interface AccordionContentProps {
  className?: string;
  children?: any;
  // injected
  _isOpen?: boolean;
  [key: string]: any;
}

export function Accordion({ type = "single", defaultValue, value, onValueChange, collapsible = false, className, children, ...rest }: AccordionProps) {
  const toArr = (v?: string | string[]) => v ? (Array.isArray(v) ? v : [v]) : [];
  const [internal, setInternal] = useState<string[]>(toArr(defaultValue));
  const controlled = value !== undefined;
  const openItems = controlled ? toArr(value) : internal;

  const handleToggle = (v: string) => {
    let next: string[];
    if (type === "single") {
      if (openItems.includes(v)) {
        next = collapsible ? [] : openItems;
      } else {
        next = [v];
      }
    } else {
      next = openItems.includes(v) ? openItems.filter(x => x !== v) : [...openItems, v];
    }
    if (!controlled) setInternal(next);
    onValueChange?.(type === "single" ? (next[0] ?? "") : next);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _openItems: openItems, _onToggle: handleToggle } };
  });

  return (
    <div className={cx("", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function AccordionItem({ value, className, children, _openItems, _onToggle, ...rest }: AccordionItemProps) {
  const isOpen = _openItems?.includes(value) ?? false;
  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onToggle: () => _onToggle?.(value) } };
  });
  return (
    <div className={cx("border-b border-gray-200", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function AccordionTrigger({ className, children, _isOpen, _onToggle, ...rest }: AccordionTriggerProps) {
  return (
    <h3 className="flex">
      <button
        type="button"
        onClick={_onToggle}
        aria-expanded={_isOpen}
        className={cx(
          "flex flex-1 items-center justify-between py-4 text-sm font-medium transition-all hover:underline text-left",
          className
        )}
        {...rest}
      >
        {children}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={cx("h-4 w-4 shrink-0 transition-transform duration-200", _isOpen ? "rotate-180" : "")}
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
    </h3>
  );
}

export function AccordionContent({ className, children, _isOpen, ...rest }: AccordionContentProps) {
  if (!_isOpen) return null;
  return (
    <div
      className={cx("overflow-hidden text-sm", className)}
      {...rest}
    >
      <div className="pb-4 pt-0">{children}</div>
    </div>
  );
}

export default Accordion;
