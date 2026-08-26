import { cx } from "zeb";

interface SeparatorProps {
  orientation?: "horizontal" | "vertical";
  decorative?: boolean;
  className?: string;
  [key: string]: any;
}

export function Separator({ orientation = "horizontal", decorative = true, className, ...rest }: SeparatorProps) {
  return (
    <div
      role={decorative ? "none" : "separator"}
      aria-orientation={decorative ? undefined : orientation}
      className={cx(
        "shrink-0 bg-gray-200",
        orientation === "horizontal" ? "h-[1px] w-full" : "h-full w-[1px]",
        className
      )}
      {...rest}
    />
  );
}

export default Separator;
