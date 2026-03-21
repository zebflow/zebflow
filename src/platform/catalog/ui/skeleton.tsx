import { cx } from "zeb";

interface SkeletonProps {
  className?: string;
  [key: string]: any;
}

export function Skeleton({ className, ...rest }: SkeletonProps) {
  return (
    <div
      className={cx("animate-pulse rounded-md bg-gray-200", className)}
      {...rest}
    />
  );
}

export default Skeleton;
