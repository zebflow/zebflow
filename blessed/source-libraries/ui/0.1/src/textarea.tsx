import { cx } from "zeb/react";

/**
 * Textarea — shadcn's multi-line input, on Zebflow's engine. Upstream's
 * `field-sizing-content` (a very new CSS property with no static height) has
 * no matching utility here, so it drops in favour of a fixed `min-h-16`,
 * like the platform's own textarea primitive already does.
 */

export function Textarea({ className, ...props }) {
  return (
    <textarea
      data-slot="textarea"
      className={cx(
        "flex min-h-16 w-full rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-xs transition-colors outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:aria-invalid:ring-destructive/40",
        className
      )}
      {...props}
    />
  );
}

export default Textarea;
