import { cx } from "zeb";

const VARIANTS = {
  default:     "bg-gray-900 text-white hover:bg-gray-800 shadow",
  destructive: "bg-red-500 text-white hover:bg-red-600 shadow-sm",
  outline:     "border border-gray-200 bg-white text-gray-900 hover:bg-gray-100 hover:text-gray-900 shadow-sm",
  secondary:   "bg-gray-100 text-gray-900 hover:bg-gray-200 shadow-sm",
  ghost:       "text-gray-900 hover:bg-gray-100 hover:text-gray-900",
  link:        "text-gray-900 underline-offset-4 hover:underline",
};

const SIZES = {
  default: "h-9 px-4 py-2",
  sm:      "h-8 rounded-md px-3 text-xs",
  lg:      "h-10 rounded-md px-8",
  icon:    "h-9 w-9",
};

interface ButtonProps {
  variant?: keyof typeof VARIANTS;
  size?: keyof typeof SIZES;
  className?: string;
  children?: any;
  type?: "button" | "submit" | "reset";
  disabled?: boolean;
  onClick?: (e: any) => void;
  [key: string]: any;
}

export function Button({
  variant = "default",
  size = "default",
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cx(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:pointer-events-none disabled:opacity-50",
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

export default Button;
