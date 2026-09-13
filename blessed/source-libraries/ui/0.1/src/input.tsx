import { cx } from "zeb/react";

/**
 * Input — shadcn's text input, on Zebflow's engine. Same class shape as
 * upstream (file/selection pseudo-elements, `aria-invalid` styling); the one
 * drop is `transition-[color,box-shadow]` (an arbitrary transition-property
 * list) in favour of the plain `transition-colors` utility the engine
 * compiles.
 */

export function Input({ className, type, ...props }) {
  return (
    <input
      type={type ?? "text"}
      data-slot="input"
      className={cx(
        "h-9 w-full min-w-0 rounded-md border border-input bg-transparent px-3 py-1 text-base shadow-xs transition-colors outline-none selection:bg-primary selection:text-primary-foreground file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm dark:bg-input/30",
        "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
        className
      )}
      {...props}
    />
  );
}

export default Input;
