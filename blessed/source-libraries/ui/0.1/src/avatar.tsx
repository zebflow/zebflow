import { cx, useState } from "zeb/react";

/**
 * Avatar — shadcn/ui's avatar, on Zebflow's engine.
 *
 * No Radix: `AvatarImage` tracks its own load failure with `useState` and
 * `onError`, rendering nothing once the image has failed so the sibling
 * `AvatarFallback` (initials) shows through — there is no automatic
 * mount-order fallback like Radix's. `size` is set as `data-size` on `Avatar`
 * itself and read back with `data-[size=…]` (allowed: same element sets and
 * reads it). Upstream also shrinks `AvatarFallback`'s text with
 * `group-data-[size=sm]/avatar:text-xs`, a named-group selector this engine
 * doesn't compile — pass `className="text-xs"` to `AvatarFallback` yourself
 * for a small avatar. `AvatarBadge`, `AvatarGroup` and `AvatarGroupCount` are
 * dropped: they lean entirely on `group-data-[…]/…`, `*:data-[slot=…]` and
 * `group-has-data-[…]` selectors that don't exist here.
 */

export function Avatar({ className, size = "default", children, ...rest }) {
  return (
    <span
      data-slot="avatar"
      data-size={size}
      className={cx(
        "relative flex size-8 shrink-0 overflow-hidden rounded-full select-none data-[size=lg]:size-10 data-[size=sm]:size-6",
        className
      )}
      {...rest}
    >
      {children}
    </span>
  );
}

export function AvatarImage({ className, onError, ...rest }) {
  const [errored, setErrored] = useState(false);
  if (errored) return null;
  return (
    <img
      data-slot="avatar-image"
      className={cx("aspect-square size-full", className)}
      onError={(e) => {
        setErrored(true);
        if (onError) onError(e);
      }}
      {...rest}
    />
  );
}

export function AvatarFallback({ className, children, ...rest }) {
  return (
    <span
      data-slot="avatar-fallback"
      className={cx("flex size-full items-center justify-center rounded-full bg-muted text-sm text-muted-foreground", className)}
      {...rest}
    >
      {children}
    </span>
  );
}

export default Avatar;
