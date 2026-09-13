import { cx, useState } from "zeb/react";

/**
 * Slider — a single-thumb range control. Upstream's Radix slider can render
 * several thumbs from a `value` array; the only control this engine has is
 * a native `<input type="range">`, which is single-valued, so this port
 * takes a plain number for `value`/`defaultValue`/`onValueChange` (a
 * documented simplification, not a bug). The track and fill are drawn with
 * real theme-token classes behind a transparent, full-size native range
 * input, which is what actually captures pointer drag and keyboard input
 * (arrow keys, Home/End) — the visuals are decoration, the input is real.
 */

export function Slider({ className, value, defaultValue, min = 0, max = 100, step = 1, onValueChange, disabled, ...props }) {
  const [internal, setInternal] = useState(defaultValue ?? min);
  const isControlled = value !== undefined;
  const current = isControlled ? value : internal;
  const pct = max > min ? ((current - min) / (max - min)) * 100 : 0;

  function handleChange(e) {
    const next = Number(e.target.value);
    if (!isControlled) setInternal(next);
    onValueChange?.(next);
  }

  return (
    <div data-slot="slider" className={cx("relative flex w-full touch-none items-center select-none", disabled ? "opacity-50" : "", className)}>
      <div data-slot="slider-track" className="relative h-1.5 w-full grow overflow-hidden rounded-full bg-muted">
        <div data-slot="slider-range" className="absolute h-full rounded-full bg-primary" style={{ width: `${pct}%` }} />
      </div>
      <div
        data-slot="slider-thumb"
        aria-hidden="true"
        className="pointer-events-none absolute block size-4 shrink-0 rounded-full border border-primary bg-background shadow-sm"
        style={{ left: `calc(${pct}% - 0.5rem)` }}
      />
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={current}
        onInput={handleChange}
        disabled={disabled}
        className="absolute inset-0 h-full w-full cursor-pointer appearance-none bg-transparent opacity-0 disabled:cursor-not-allowed"
        {...props}
      />
    </div>
  );
}

export default Slider;
