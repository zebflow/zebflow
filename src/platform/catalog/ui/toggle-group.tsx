import { cx } from "zeb";
import { useState } from "zeb";

interface ToggleGroupProps {
  type?: "single" | "multiple";
  value?: string | string[];
  defaultValue?: string | string[];
  onValueChange?: (value: string | string[]) => void;
  variant?: "default" | "outline";
  size?: "default" | "sm" | "lg";
  disabled?: boolean;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface ToggleGroupItemProps {
  value: string;
  disabled?: boolean;
  className?: string;
  children?: any;
  // injected
  _type?: string;
  _activeValues?: string[];
  _onToggle?: (v: string) => void;
  _variant?: string;
  _size?: string;
  [key: string]: any;
}

const ITEM_VARIANTS = {
  default: "bg-transparent hover:bg-gray-100 hover:text-gray-900 data-[state=on]:bg-gray-100 data-[state=on]:text-gray-900",
  outline: "border border-gray-200 bg-transparent hover:bg-gray-100 hover:text-gray-900 data-[state=on]:bg-gray-100 data-[state=on]:text-gray-900",
};

const ITEM_SIZES = {
  default: "h-9 px-3 min-w-9",
  sm:      "h-8 px-2 min-w-8",
  lg:      "h-10 px-5 min-w-10",
};

export function ToggleGroup({ type = "single", value, defaultValue, onValueChange, variant = "default", size = "default", disabled, className, children, ...rest }: ToggleGroupProps) {
  const toArr = (v?: string | string[]) => v ? (Array.isArray(v) ? v : [v]) : [];
  const [internal, setInternal] = useState<string[]>(toArr(defaultValue));
  const controlled = value !== undefined;
  const active = controlled ? toArr(value) : internal;

  const handleToggle = (v: string) => {
    if (disabled) return;
    let next: string[];
    if (type === "single") {
      next = active.includes(v) ? [] : [v];
    } else {
      next = active.includes(v) ? active.filter(x => x !== v) : [...active, v];
    }
    if (!controlled) setInternal(next);
    onValueChange?.(type === "single" ? (next[0] ?? "") : next);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _type: type, _activeValues: active, _onToggle: handleToggle, _variant: variant, _size: size } };
  });

  return (
    <div
      role="group"
      className={cx("flex items-center justify-center gap-1", className)}
      {...rest}
    >
      {enhanced}
    </div>
  );
}

export function ToggleGroupItem({ value, disabled, className, children, _activeValues, _onToggle, _variant = "default", _size = "default", ...rest }: ToggleGroupItemProps) {
  const isOn = _activeValues?.includes(value) ?? false;
  return (
    <button
      type="button"
      data-state={isOn ? "on" : "off"}
      disabled={disabled}
      onClick={() => _onToggle?.(value)}
      className={cx(
        "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:pointer-events-none disabled:opacity-50",
        ITEM_VARIANTS[_variant as keyof typeof ITEM_VARIANTS] ?? ITEM_VARIANTS.default,
        ITEM_SIZES[_size as keyof typeof ITEM_SIZES] ?? ITEM_SIZES.default,
        className
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export default ToggleGroup;
