import { cx } from "zeb";

interface SwitchProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  disabled?: boolean;
  id?: string;
  className?: string;
  [key: string]: any;
}

export function Switch({ checked, defaultChecked, onCheckedChange, disabled, id, className, ...rest }: SwitchProps) {
  const handleChange = (e: any) => {
    onCheckedChange?.(e.target.checked);
  };

  return (
    <label
      className={cx(
        "relative inline-flex h-6 w-11 cursor-pointer items-center rounded-full transition-colors",
        checked ? "bg-gray-900" : "bg-gray-200",
        disabled ? "cursor-not-allowed opacity-50" : "",
        className
      )}
    >
      <input
        type="checkbox"
        id={id}
        checked={checked}
        defaultChecked={defaultChecked}
        onChange={handleChange}
        disabled={disabled}
        className="sr-only peer"
        {...rest}
      />
      <span
        className={cx(
          "absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform",
          checked ? "translate-x-5" : "translate-x-0"
        )}
      />
    </label>
  );
}

export default Switch;
