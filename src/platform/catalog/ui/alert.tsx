import { cx } from "zeb";

const VARIANTS = {
  default:     "bg-white text-gray-900 border-gray-200",
  destructive: "border-red-500/50 text-red-700 bg-red-50 [&>svg]:text-red-700",
};

interface AlertProps {
  variant?: keyof typeof VARIANTS;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface AlertTitleProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

interface AlertDescriptionProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Alert({ variant = "default", className, children, ...rest }: AlertProps) {
  return (
    <div
      role="alert"
      className={cx(
        "relative w-full rounded-lg border p-4 [&>svg+div]:translate-y-[-3px] [&>svg]:absolute [&>svg]:left-4 [&>svg]:top-4 [&>svg~*]:pl-7",
        VARIANTS[variant] ?? VARIANTS.default,
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function AlertTitle({ className, children, ...rest }: AlertTitleProps) {
  return (
    <h5
      className={cx("mb-1 font-medium leading-none tracking-tight", className)}
      {...rest}
    >
      {children}
    </h5>
  );
}

export function AlertDescription({ className, children, ...rest }: AlertDescriptionProps) {
  return (
    <div
      className={cx("text-sm [&_p]:leading-relaxed", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Alert;
