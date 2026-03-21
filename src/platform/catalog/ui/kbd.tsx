import { cx } from "zeb";

interface KbdProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Kbd({ className, children, ...rest }: KbdProps) {
  return (
    <kbd
      className={cx(
        "pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border border-gray-200 bg-gray-50 px-1.5 font-mono text-[10px] font-medium text-gray-600",
        className
      )}
      {...rest}
    >
      {children}
    </kbd>
  );
}

export default Kbd;
