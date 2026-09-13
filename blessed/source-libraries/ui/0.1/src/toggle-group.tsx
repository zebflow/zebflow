import { cx, createContext, useContext, useRef, useState } from "zeb/react";
import { Toggle } from "zeb/ui/toggle";

/**
 * ToggleGroup — a connected row of Toggle buttons where at most one
 * (`type="single"`) or several (`type="multiple"`) stay pressed. Composed
 * through context exactly as upstream Radix does: `ToggleGroupItem` reads
 * pressed state, variant/size and the toggle handler from
 * `ToggleGroupContext` no matter how deep it is nested. Controlled via
 * `value`/`onValueChange` on the group (upstream names), uncontrolled via
 * `defaultValue`. Keyboard follows Radix's actual behaviour (a roving-focus
 * toolbar, not a radio group): ArrowLeft/Right move DOM focus to the
 * previous/next enabled item and wrap at the ends; they do not themselves
 * change selection — Space/Enter/click still toggles, same as a plain
 * button. Only one item sits in the tab sequence at a time (the pressed one,
 * or the first enabled item when nothing is pressed); the rest are
 * `tabIndex={-1}`, found by walking render order rather than the DOM so it
 * is correct on the very first server render.
 */

const ToggleGroupContext = createContext(null);

export function ToggleGroup({ type = "single", value, defaultValue, onValueChange, variant = "default", size = "default", disabled, className, children, ...props }) {
  const toArray = (v) => (v === undefined || v === null ? [] : Array.isArray(v) ? v : [v]);
  const [internal, setInternal] = useState(toArray(defaultValue));
  const isControlled = value !== undefined;
  const active = isControlled ? toArray(value) : internal;
  const orderRef = useRef([]);
  orderRef.current = [];

  function handleToggle(itemValue) {
    if (disabled) return;
    let next;
    if (type === "multiple") {
      next = active.includes(itemValue) ? active.filter((v) => v !== itemValue) : [...active, itemValue];
    } else {
      next = active.includes(itemValue) ? [] : [itemValue];
    }
    if (!isControlled) setInternal(next);
    onValueChange?.(type === "multiple" ? next : next[0] ?? "");
  }

  // Called synchronously from each Item's own render, in document order, so
  // the first still-enabled item (with nothing pressed) is knowable without
  // touching the DOM — this is what gives it tabIndex 0 on first paint.
  function registerAndIsFirstEnabled(itemValue, itemDisabled) {
    const isFirst = !itemDisabled && !orderRef.current.some((it) => !it.disabled);
    orderRef.current.push({ value: itemValue, disabled: itemDisabled });
    return isFirst;
  }

  function focusStep(currentEl, dir) {
    const group = currentEl?.closest?.('[data-slot="toggle-group"]');
    if (!group) return;
    const buttons = Array.from(group.querySelectorAll('[data-slot="toggle-group-item"]')).filter((b) => !b.disabled);
    if (!buttons.length) return;
    const idx = buttons.indexOf(currentEl);
    const next = buttons[(idx + dir + buttons.length) % buttons.length];
    next?.focus();
  }

  const ctx = {
    variant,
    size,
    disabled,
    hasActive: active.length > 0,
    isActive: (v) => active.includes(v),
    onToggle: handleToggle,
    registerAndIsFirstEnabled,
    focusStep,
  };

  return (
    <div role="group" data-slot="toggle-group" data-variant={variant} data-size={size} className={cx("flex w-fit items-center rounded-md", className)} {...props}>
      <ToggleGroupContext.Provider value={ctx}>{children}</ToggleGroupContext.Provider>
    </div>
  );
}

export function ToggleGroupItem({ value, className, variant, size, disabled, onKeyDown, ...props }) {
  const ctx = useContext(ToggleGroupContext);
  const pressed = ctx?.isActive?.(value) ?? false;
  const isDisabled = disabled || ctx?.disabled;
  const isFirstEnabled = ctx?.registerAndIsFirstEnabled?.(value, isDisabled) ?? false;
  const tabIndex = pressed || (!ctx?.hasActive && isFirstEnabled) ? 0 : -1;

  function handleKeyDown(e) {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      ctx?.focusStep?.(e.currentTarget, 1);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      ctx?.focusStep?.(e.currentTarget, -1);
    }
    onKeyDown?.(e);
  }

  return (
    <Toggle
      data-slot="toggle-group-item"
      variant={variant ?? ctx?.variant}
      size={size ?? ctx?.size}
      pressed={pressed}
      disabled={isDisabled}
      tabIndex={tabIndex}
      onPressedChange={() => ctx?.onToggle?.(value)}
      onKeyDown={handleKeyDown}
      className={cx(
        "min-w-0 shrink-0 rounded-none border-l-0 first:rounded-l-md first:border-l last:rounded-r-md focus:z-10 focus-visible:z-10",
        className
      )}
      {...props}
    />
  );
}

export default ToggleGroup;
