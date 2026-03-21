import { cx } from "zeb";

interface CheckboxProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  onChange?: (e: any) => void;
  disabled?: boolean;
  id?: string;
  className?: string;
  [key: string]: any;
}

export function Checkbox({ checked, defaultChecked, onCheckedChange, onChange, disabled, id, className, ...rest }: CheckboxProps) {
  const handleChange = (e: any) => {
    onChange?.(e);
    onCheckedChange?.(e.target.checked);
  };

  return (
    <input
      type="checkbox"
      id={id}
      checked={checked}
      defaultChecked={defaultChecked}
      onChange={handleChange}
      disabled={disabled}
      className={cx(
        "h-4 w-4 shrink-0 rounded border border-gray-900 shadow focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:cursor-not-allowed disabled:opacity-50 accent-gray-900",
        className
      )}
      {...rest}
    />
  );
}

export default Checkbox;
