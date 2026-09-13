/**
 * AspectRatio — shadcn/ui's aspect-ratio box, on Zebflow's engine.
 *
 * No Radix: a plain `<div>` sets the CSS `aspect-ratio` property inline
 * (a computed value from the `ratio` prop, not a class) and lets content
 * fill it — the same effect Radix's primitive achieves.
 */

export function AspectRatio({ ratio = 1, style, children, ...rest }) {
  return (
    <div data-slot="aspect-ratio" style={{ position: "relative", width: "100%", aspectRatio: String(ratio), ...style }} {...rest}>
      {children}
    </div>
  );
}

export default AspectRatio;
