import { cx } from "zeb";

interface ScrollAreaProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

interface ScrollBarProps {
  orientation?: "vertical" | "horizontal";
  className?: string;
  [key: string]: any;
}

export function ScrollArea({ className, children, ...rest }: ScrollAreaProps) {
  return (
    <div
      className={cx("relative overflow-hidden", className)}
      {...rest}
    >
      <div className="h-full w-full overflow-auto [scrollbar-width:thin] [scrollbar-color:rgb(203_213_225)_transparent] [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-gray-300">
        {children}
      </div>
    </div>
  );
}

export function ScrollBar({ orientation = "vertical", className, ...rest }: ScrollBarProps) {
  return (
    <div
      className={cx(
        "flex touch-none select-none transition-colors",
        orientation === "vertical" ? "h-full w-2.5 border-l border-l-transparent p-[1px]" : "h-2.5 flex-col border-t border-t-transparent p-[1px]",
        className
      )}
      {...rest}
    />
  );
}

export default ScrollArea;
