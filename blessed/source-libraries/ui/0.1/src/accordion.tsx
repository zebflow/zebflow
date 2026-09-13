import { cx, createContext, useContext } from "zeb/react";
import { useControllable } from "zeb/ui/hooks";

/**
 * Accordion — shadcn/ui's accordion, on Zebflow's engine.
 *
 * Built on native `<details>`/`<summary>` so a single item opens and closes
 * correctly before hydration ever runs. `value`/`defaultValue`/
 * `onValueChange` are real: `Accordion` holds the open value (a string for
 * `type="single"`, an array of strings for `type="multiple"`) in an
 * `AccordionContext`, the same way Radix's `Accordion.Root` hands its state
 * down — each `AccordionItem` reads whether *it* is open from context by its
 * own `value` and sets the native `open` attribute accordingly, so a nested
 * accordion's items never see an outer accordion's context (each `Provider`
 * only reaches its own descendants — no DOM querying, no cross-talk).
 * `type="single"` closes a sibling by re-deriving its `open` prop to `false`
 * on the next render; clicking the already-open item when not `collapsible`
 * reverts the browser's native toggle synchronously so it refuses to close.
 * `type="multiple"` just keeps every item's value in an array. The chevron
 * rotates with `group-open:rotate-180` off a `group` class on the
 * `<details>` itself; there is no need to thread the open flag to the icon
 * separately.
 */

const AccordionContext = createContext({ isOpen: () => false, toggle: () => true });

export function Accordion({ type = "single", collapsible = false, value, defaultValue, onValueChange, className, children, ...props }) {
  const [current, setValue] = useControllable(value, defaultValue ?? (type === "multiple" ? [] : ""), onValueChange);

  const isOpen = (v) => (type === "multiple" ? (current || []).includes(v) : current === v);

  /** Returns whether the change was accepted, so the item can revert the DOM if not. */
  const toggle = (v) => {
    if (type === "multiple") {
      const arr = current || [];
      setValue(arr.includes(v) ? arr.filter((x) => x !== v) : [...arr, v]);
      return true;
    }
    if (current === v) {
      if (!collapsible) return false;
      setValue("");
      return true;
    }
    setValue(v);
    return true;
  };

  return (
    <AccordionContext.Provider value={{ isOpen, toggle }}>
      <div data-slot="accordion" className={cx("", className)} {...props}>
        {children}
      </div>
    </AccordionContext.Provider>
  );
}

export function AccordionItem({ value, className, children, ...props }) {
  const ctx = useContext(AccordionContext);
  const open = ctx.isOpen(value);
  const onToggle = (e) => {
    const next = e.target.open;
    if (next === open) return;
    if (!ctx.toggle(value)) e.target.open = open;
  };
  return (
    <details
      data-slot="accordion-item"
      data-value={value}
      open={open}
      onToggle={onToggle}
      className={cx("group", "border-b border-border last:border-b-0", className)}
      {...props}
    >
      {children}
    </details>
  );
}

export function AccordionTrigger({ className, children, ...props }) {
  return (
    <summary
      data-slot="accordion-trigger"
      style={{ listStyle: "none" }}
      className={cx(
        "flex flex-1 cursor-pointer items-start justify-between gap-4 py-4 text-left text-sm font-medium outline-none transition-all hover:underline focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
        className
      )}
      {...props}
    >
      {children}
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="pointer-events-none size-4 shrink-0 translate-y-0.5 text-muted-foreground transition-transform duration-200 group-open:rotate-180"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </summary>
  );
}

export function AccordionContent({ className, children, ...props }) {
  return (
    <div data-slot="accordion-content" className="overflow-hidden text-sm" {...props}>
      <div className={cx("pb-4 pt-0", className)}>{children}</div>
    </div>
  );
}

export default Accordion;
