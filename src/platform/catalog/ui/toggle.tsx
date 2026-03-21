import { cx } from "zeb";

const VARIANTS = {
  default: "bg-transparent hover:bg-gray-100 hover:text-gray-900 data-[state=on]:bg-gray-100 data-[state=on]:text-gray-900",
  outline: "border border-gray-200 bg-transparent hover:bg-gray-100 hover:text-gray-900 data-[state=on]:bg-gray-100 data-[state=on]:text-gray-900",
};

const SIZES = {
  default: "h-9 px-3 min-w-9",
  sm:      "h-8 px-2 min-w-8",
  lg:      "h-10 px-5 min-w-10",
};

interface ToggleProps {
  variant?: keyof typeof VARIANTS;
  size?: keyof typeof SIZES;
  pressed?: boolean;
  defaultPressed?: boolean;
  onPressedChange?: (pressed: boolean) => void;
  disabled?: boolean;
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Toggle({ variant = "default", size = "default", pressed, defaultPressed, onPressedChange, disabled, className, children, ...rest }: ToggleProps) {
  const handleClick = () => {
    if (!disabled) onPressedChange?.(!pressed);
  };

  return (
    <button
      type="button"
      role="button"
      aria-pressed={pressed}
      data-state={pressed ? "on" : "off"}
      disabled={disabled}
      onClick={handleClick}
      className={cx(
        "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:pointer-events-none disabled:opacity-50",
        VARIANTS[variant] ?? VARIANTS.default,
        SIZES[size] ?? SIZES.default,
        className
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export default Toggle;
