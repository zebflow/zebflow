import { cx } from "zeb";

interface CardProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Card({ className, children, ...rest }: CardProps) {
  return (
    <div
      className={cx(
        "rounded-xl border border-gray-200 bg-white text-gray-900 shadow",
        className
      )}
      {...rest}
    >
      {children}
    </div>
  );
}

export function CardHeader({ className, children, ...rest }: CardProps) {
  return (
    <div className={cx("flex flex-col space-y-1.5 p-6", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardTitle({ className, children, ...rest }: CardProps) {
  return (
    <h3
      className={cx("font-semibold leading-none tracking-tight", className)}
      {...rest}
    >
      {children}
    </h3>
  );
}

export function CardDescription({ className, children, ...rest }: CardProps) {
  return (
    <p className={cx("text-sm text-gray-500", className)} {...rest}>
      {children}
    </p>
  );
}

export function CardContent({ className, children, ...rest }: CardProps) {
  return (
    <div className={cx("p-6 pt-0", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardFooter({ className, children, ...rest }: CardProps) {
  return (
    <div className={cx("flex items-center p-6 pt-0", className)} {...rest}>
      {children}
    </div>
  );
}

export default Card;
