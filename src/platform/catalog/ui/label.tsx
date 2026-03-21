import { cx } from "zeb";

interface LabelProps {
  htmlFor?: string;
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Label({ className, htmlFor, children, ...rest }: LabelProps) {
  return (
    <label
      htmlFor={htmlFor}
      className={cx(
        "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className
      )}
      {...rest}
    >
      {children}
    </label>
  );
}

export default Label;
