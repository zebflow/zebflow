import { cx, createContext, useContext, useEffect, useId, useRef } from "zeb/react";
import { useControllable } from "zeb/ui/hooks";

/**
 * Tabs — shadcn/ui's tabs, on Zebflow's engine.
 *
 * `Tabs` holds the active value — controlled via `value`/`onValueChange`,
 * uncontrolled off `defaultValue` — in a `TabsContext` and hands it straight
 * to `TabsList`/`TabsTrigger`/`TabsContent` at any depth, the same way
 * Radix's `TabsPrimitive.Root` does: a shadcn snippet's nesting works
 * unchanged, with no render-prop and no required `active`/`onClick` on the
 * sub-parts (they remain as optional overrides for a caller that wants to
 * drive its own state). `Tabs` also mints one `useId()` per instance so
 * every trigger/panel pair gets matching, collision-free `id` /
 * `aria-controls` / `aria-labelledby`.
 *
 * Arrow/Home/End keys move focus across `[role="tab"]` triggers via one
 * delegated `onKeyDown` on `TabsList` (no per-trigger index needed). Roving
 * `tabIndex` normally follows the active trigger; if no trigger matches the
 * current value at all (e.g. `defaultValue` was omitted), a render effect on
 * `TabsList` makes the first enabled trigger reachable instead, so the
 * tablist is never entirely unreachable by keyboard. `TabsContent` stays
 * mounted and toggles the native `hidden` attribute instead of unmounting,
 * so an inactive panel's own state (scroll position, form input, …)
 * survives switching away and back.
 */

const TabsContext = createContext({ value: "", setValue: () => {}, baseId: "" });

export function Tabs({ value, defaultValue, onValueChange, className, children, ...props }) {
  const [current, setValue] = useControllable(value, defaultValue ?? "", onValueChange);
  const baseId = useId();
  return (
    <TabsContext.Provider value={{ value: current, setValue, baseId }}>
      <div data-slot="tabs" className={cx("flex flex-col gap-2", className)} {...props}>
        {children}
      </div>
    </TabsContext.Provider>
  );
}

export function TabsList({ className, children, ...props }) {
  const ref = useRef(null);

  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const tabs = Array.from(root.querySelectorAll('[role="tab"]'));
    if (tabs.length === 0 || tabs.some((t) => t.getAttribute("aria-selected") === "true")) return;
    const first = tabs.find((t) => !t.disabled) ?? tabs[0];
    tabs.forEach((t) => {
      t.tabIndex = t === first ? 0 : -1;
    });
  });

  const onKeyDown = (e) => {
    if (!["ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    const tabs = Array.from(e.currentTarget.querySelectorAll('[role="tab"]:not(:disabled)'));
    if (tabs.length === 0) return;
    const from = tabs.indexOf(document.activeElement);
    let to = from;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") to = from < 0 ? 0 : (from + 1) % tabs.length;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") to = from < 0 ? tabs.length - 1 : (from - 1 + tabs.length) % tabs.length;
    else if (e.key === "Home") to = 0;
    else if (e.key === "End") to = tabs.length - 1;
    e.preventDefault();
    tabs[to].focus();
    tabs[to].click();
  };

  return (
    <div
      ref={ref}
      role="tablist"
      data-slot="tabs-list"
      onKeyDown={onKeyDown}
      className={cx(
        "inline-flex h-9 w-fit items-center justify-center rounded-lg bg-muted p-[3px] text-muted-foreground",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}

export function TabsTrigger({ value, active, disabled, className, children, onClick, id, ...props }) {
  const ctx = useContext(TabsContext);
  const isActive = active !== undefined ? active : ctx.value === value;
  const triggerId = id ?? `${ctx.baseId}-trigger-${value}`;
  const handleClick = onClick ?? (() => ctx.setValue(value));
  return (
    <button
      type="button"
      role="tab"
      id={triggerId}
      aria-controls={`${ctx.baseId}-content-${value}`}
      data-slot="tabs-trigger"
      data-state={isActive ? "active" : "inactive"}
      aria-selected={isActive}
      tabIndex={isActive ? 0 : -1}
      disabled={disabled}
      onClick={disabled ? undefined : handleClick}
      className={cx(
        "inline-flex flex-1 items-center justify-center gap-1.5 whitespace-nowrap rounded-md border border-transparent px-2 py-1 text-sm font-medium text-foreground/60 transition-all hover:text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-xs dark:data-[state=active]:border-input dark:data-[state=active]:bg-input/30 dark:text-muted-foreground dark:hover:text-foreground",
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}

export function TabsContent({ value, active, id, className, children, ...props }) {
  const ctx = useContext(TabsContext);
  const isActive = active !== undefined ? active : ctx.value === value;
  return (
    <div
      role="tabpanel"
      id={id ?? `${ctx.baseId}-content-${value}`}
      aria-labelledby={`${ctx.baseId}-trigger-${value}`}
      hidden={!isActive}
      data-slot="tabs-content"
      data-state={isActive ? "active" : "inactive"}
      className={cx("flex-1 outline-none", className)}
      {...props}
    >
      {children}
    </div>
  );
}

export default Tabs;
