import { cx } from "zeb";
import { useState } from "zeb";

interface RadioGroupProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface RadioGroupItemProps {
  value: string;
  id?: string;
  disabled?: boolean;
  className?: string;
  // injected by RadioGroup context via cloneElement pattern
  _groupValue?: string;
  _onGroupChange?: (v: string) => void;
  [key: string]: any;
}

export function RadioGroup({ value, defaultValue, onValueChange, className, children, ...rest }: RadioGroupProps) {
  const [internal, setInternal] = useState(defaultValue ?? "");
  const controlled = value !== undefined;
  const current = controlled ? value : internal;

  const handleChange = (v: string) => {
    if (!controlled) setInternal(v);
    onValueChange?.(v);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any, i: number) => {
    if (!child) return child;
    return { ...child, props: { ...child.props, _groupValue: current, _onGroupChange: handleChange } };
  });

  return (
    <div role="radiogroup" className={cx("grid gap-2", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function RadioGroupItem({ value, id, disabled, className, _groupValue, _onGroupChange, ...rest }: RadioGroupItemProps) {
  const checked = _groupValue === value;
  return (
    <input
      type="radio"
      id={id}
      value={value}
      checked={checked}
      disabled={disabled}
      onChange={() => _onGroupChange?.(value)}
      className={cx(
        "h-4 w-4 rounded-full border border-gray-900 text-gray-900 shadow focus:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:cursor-not-allowed disabled:opacity-50 accent-gray-900",
        className
      )}
      {...rest}
    />
  );
}

export default RadioGroup;
