import { cx } from "zeb/react";
import { Label } from "zeb/ui/label";

/**
 * Field — shadcn's form-layout family (FieldSet, FieldLegend, FieldGroup,
 * Field, FieldContent, FieldLabel, FieldTitle, FieldDescription,
 * FieldSeparator, FieldError), on Zebflow's engine. Three upstream pieces
 * don't survive this engine's Tailwind subset:
 *   - `orientation="responsive"` used a named `@container/field-group` query
 *     (`@md/field-group:...`); there is no container-query variant here, so
 *     only `"vertical"` and `"horizontal"` are offered.
 *   - the `has-[>[data-slot=...]]:` / `[&>[data-slot=...]]:` density and
 *     child-selector hooks (sizing a FieldSet around a checkbox group, a
 *     FieldLabel that grows a border around a nested Field) all reach into
 *     children through selectors this engine doesn't compile; adjust spacing
 *     with `className` on the piece that needs it instead.
 *   - FieldSeparator drew its rule with the `Separator` primitive; that isn't
 *     part of this group, so it draws its own `border-t` line instead of
 *     importing a sibling.
 * FieldDescription also drops `nth-last-2:` and the `[[data-variant=legend]+&]`
 * / `[&>a]:` selectors — not in the engine's variant set.
 */

const ORIENTATION = {
  vertical: "flex-col",
  horizontal: "flex-row items-center",
};

export function FieldSet({ className, ...props }) {
  return <fieldset data-slot="field-set" className={cx("flex flex-col gap-6", className)} {...props} />;
}

const LEGEND_VARIANTS = {
  legend: "text-base",
  label: "text-sm",
};

export function FieldLegend({ className, variant = "legend", ...props }) {
  return (
    <legend
      data-slot="field-legend"
      data-variant={variant}
      className={cx("mb-3 font-medium", LEGEND_VARIANTS[variant] ?? LEGEND_VARIANTS.legend, className)}
      {...props}
    />
  );
}

export function FieldGroup({ className, ...props }) {
  return <div data-slot="field-group" className={cx("flex w-full flex-col gap-7", className)} {...props} />;
}

export function Field({ className, orientation = "vertical", ...props }) {
  return (
    <div
      role="group"
      data-slot="field"
      data-orientation={orientation}
      className={cx("flex w-full gap-3 data-[invalid=true]:text-destructive", ORIENTATION[orientation] ?? ORIENTATION.vertical, className)}
      {...props}
    />
  );
}

export function FieldContent({ className, ...props }) {
  return <div data-slot="field-content" className={cx("flex flex-1 flex-col gap-1.5 leading-snug", className)} {...props} />;
}

export function FieldLabel({ className, ...props }) {
  return <Label data-slot="field-label" className={cx("flex w-fit gap-2 leading-snug", className)} {...props} />;
}

export function FieldTitle({ className, ...props }) {
  return (
    <div
      data-slot="field-title"
      className={cx("flex w-fit items-center gap-2 text-sm font-medium leading-snug", className)}
      {...props}
    />
  );
}

export function FieldDescription({ className, ...props }) {
  return (
    <p
      data-slot="field-description"
      className={cx("text-sm leading-normal font-normal text-muted-foreground last:mt-0", className)}
      {...props}
    />
  );
}

export function FieldSeparator({ children, className, ...props }) {
  return (
    <div data-slot="field-separator" data-content={children ? "true" : "false"} className={cx("relative -my-2 h-5 text-sm", className)} {...props}>
      <div aria-hidden="true" className="absolute inset-0 flex items-center">
        <div className="w-full border-t border-border" />
      </div>
      {children ? (
        <span data-slot="field-separator-content" className="relative mx-auto block w-fit bg-background px-2 text-muted-foreground">
          {children}
        </span>
      ) : null}
    </div>
  );
}

export function FieldError({ className, children, errors, ...props }) {
  let content = children;
  if (!content && errors && errors.length) {
    const seen = new Map();
    for (const error of errors) {
      if (error && error.message) seen.set(error.message, error);
    }
    const unique = Array.from(seen.values());
    content =
      unique.length === 1 ? (
        unique[0].message
      ) : (
        <ul className="ml-4 flex list-disc flex-col gap-1">
          {unique.map((error, index) => (error?.message ? <li key={index}>{error.message}</li> : null))}
        </ul>
      );
  }
  if (!content) return null;
  return (
    <div role="alert" data-slot="field-error" className={cx("text-sm font-normal text-destructive", className)} {...props}>
      {content}
    </div>
  );
}

export default Field;
