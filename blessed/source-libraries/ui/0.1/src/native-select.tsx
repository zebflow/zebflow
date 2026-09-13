import { cx } from "zeb/react";

/**
 * NativeSelect — a real `<select>`, styled like the other form controls,
 * with a decorative chevron overlay. `<option>`/`<optgroup>` accept almost
 * no CSS in any browser, so their classes only ever set the OS system
 * colours `Canvas`/`CanvasText` (matching whichever native popup style the
 * platform draws) — not a themed surface, an OS one, same as upstream.
 */

export function NativeSelect({ className, size = "default", disabled, ...props }) {
  return (
    <div className={cx("relative w-fit", disabled ? "opacity-50" : "")} data-slot="native-select-wrapper">
      <select
        data-slot="native-select"
        data-size={size}
        disabled={disabled}
        className={cx(
          "h-9 w-full min-w-0 appearance-none rounded-md border border-input bg-transparent px-3 py-2 pr-9 text-sm shadow-xs transition-colors outline-none disabled:pointer-events-none disabled:cursor-not-allowed",
          "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
          "aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
          size === "sm" ? "h-8 py-1" : "",
          "dark:bg-input/30 dark:hover:bg-input/50",
          className
        )}
        {...props}
      />
      <svg
        aria-hidden="true"
        data-slot="native-select-icon"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="pointer-events-none absolute right-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground opacity-50"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </div>
  );
}

export function NativeSelectOption({ className, ...props }) {
  return <option data-slot="native-select-option" className={cx("bg-[Canvas] text-[CanvasText]", className)} {...props} />;
}

export function NativeSelectOptGroup({ className, ...props }) {
  return <optgroup data-slot="native-select-optgroup" className={cx("bg-[Canvas] text-[CanvasText]", className)} {...props} />;
}

export default NativeSelect;
