import { cx } from "zeb";
import { useState } from "zeb";

interface TabsProps {
  defaultValue?: string;
  value?: string;
  onValueChange?: (value: string) => void;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface TabsListProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

interface TabsTriggerProps {
  value: string;
  disabled?: boolean;
  className?: string;
  children?: any;
  // injected by Tabs
  _activeTab?: string;
  _onTabChange?: (v: string) => void;
  [key: string]: any;
}

interface TabsContentProps {
  value: string;
  className?: string;
  children?: any;
  // injected by Tabs
  _activeTab?: string;
  [key: string]: any;
}

export function Tabs({ defaultValue, value, onValueChange, className, children, ...rest }: TabsProps) {
  const [internal, setInternal] = useState(defaultValue ?? "");
  const controlled = value !== undefined;
  const active = controlled ? value : internal;

  const handleChange = (v: string) => {
    if (!controlled) setInternal(v);
    onValueChange?.(v);
  };

  const inject = (child: any): any => {
    if (!child || typeof child !== "object") return child;
    const props = child.props ?? {};
    const newProps: any = { ...props, _activeTab: active, _onTabChange: handleChange };
    if (Array.isArray(props.children)) {
      newProps.children = props.children.map(inject);
    } else if (props.children && typeof props.children === "object") {
      newProps.children = inject(props.children);
    }
    return { ...child, props: newProps };
  };

  const items = Array.isArray(children) ? children : [children];
  return (
    <div className={cx("", className)} {...rest}>
      {items.map(inject)}
    </div>
  );
}

export function TabsList({ className, children, _activeTab, _onTabChange, ...rest }: TabsListProps & { _activeTab?: string; _onTabChange?: (v: string) => void }) {
  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _activeTab, _onTabChange } };
  });
  return (
    <div
      role="tablist"
      className={cx(
        "inline-flex h-9 items-center justify-center rounded-lg bg-gray-100 p-1 text-gray-500",
        className
      )}
      {...rest}
    >
      {enhanced}
    </div>
  );
}

export function TabsTrigger({ value, disabled, className, children, _activeTab, _onTabChange, ...rest }: TabsTriggerProps) {
  const active = _activeTab === value;
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      disabled={disabled}
      onClick={() => !disabled && _onTabChange?.(value)}
      className={cx(
        "inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1 text-sm font-medium ring-offset-white transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gray-950 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        active ? "bg-white text-gray-900 shadow" : "text-gray-500 hover:text-gray-900",
        className
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export function TabsContent({ value, className, children, _activeTab, ...rest }: TabsContentProps) {
  if (_activeTab !== value) return null;
  return (
    <div
      role="tabpanel"
      className={cx(
        "mt-2 ring-offset-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-gray-950 focus-visible:ring-offset-2",
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Tabs;
