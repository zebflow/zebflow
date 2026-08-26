import { cx } from "zeb";

interface ProgressProps {
  value?: number;
  max?: number;
  className?: string;
  [key: string]: any;
}

export function Progress({ value = 0, max = 100, className, ...rest }: ProgressProps) {
  const pct = Math.min(Math.max((value / max) * 100, 0), 100);
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={max}
      aria-valuenow={value}
      className={cx(
        "relative h-2 w-full overflow-hidden rounded-full bg-gray-100",
        className
      )}
      {...rest}
    >
      <div
        className="h-full bg-gray-900 transition-all duration-300 ease-in-out"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

export default Progress;
