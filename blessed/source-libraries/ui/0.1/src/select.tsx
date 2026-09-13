import { cx, createContext, useContext, useRef, useId, useEffect, createPortal } from "zeb/react";
import { useControllable, useClickAway, useEscape, useAnchoredPosition } from "zeb/ui/hooks";

/**
 * Select — shadcn's select, on Zebflow's engine. Upstream's is a listbox,
 * not the native element, so that is what this is too: `role="listbox"` /
 * `role="option"` with real keyboard support.
 *
 * `Select`/`SelectTrigger`/`SelectContent`/`SelectItem` compose through
 * `createContext` instead of `child.type` detection or manual prop
 * threading: `Select` owns both whether the listbox is open *and* the
 * selected value (controlled via `value`/`onValueChange`, uncontrolled via
 * `defaultValue`, upstream's names either way), and hands both down through
 * context the way `RadioGroup` injects `checked` onto its items — every
 * `SelectItem` reads `selected`/`onSelect` from context unless it is given
 * its own (an optional override), and `SelectValue` falls back to the
 * context value when not given one explicitly. (`SelectValue` shows that
 * raw value, not the selected item's rendered label — there is no item
 * registry to look a label up from — so pass `value` explicitly for a
 * prettier display.)
 *
 * On open, focus moves into the panel — onto the selected option, or the
 * first one — instead of staying on the trigger with the keys doing
 * nothing; ArrowUp/ArrowDown on the trigger itself opens the listbox (which
 * then focuses in the same way). `SelectTrigger` carries `aria-expanded`,
 * `aria-controls`, and `aria-haspopup="listbox"`. `SelectContent` portals to
 * `document.body` in the browser, rendering inline during SSR; click-away
 * checks both the trigger and the (portaled) panel and is gated on
 * `isOpen`.
 */

const ITEM_SELECTOR = '[role="option"]:not([aria-disabled="true"])';

const SelectContext = createContext(null);

function useSelectContext(name) {
  const ctx = useContext(SelectContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <Select>`);
  return ctx;
}

export function Select({ value, defaultValue, onValueChange, open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const [currentValue, setValue] = useControllable(value, defaultValue ?? "", onValueChange);
  const anchorRef = useRef(null);
  const panelRef = useRef(null);
  const listboxId = useId();

  useClickAway([anchorRef, panelRef], () => setIsOpen(false), isOpen);
  useEscape(isOpen, () => setIsOpen(false));

  function selectValue(v) {
    setValue(v);
    setIsOpen(false);
  }

  return (
    <SelectContext.Provider value={{ isOpen, setIsOpen, value: currentValue, selectValue, anchorRef, panelRef, listboxId }}>
      <div data-slot="select" className="block w-full">
        {children}
      </div>
    </SelectContext.Provider>
  );
}

export function SelectTrigger({ className, children, onClick, onKeyDown, ...props }) {
  const { isOpen, setIsOpen, anchorRef, listboxId } = useSelectContext("SelectTrigger");

  function handleClick(event) {
    if (typeof onClick === "function") onClick(event);
    if (!event.defaultPrevented) setIsOpen(!isOpen);
  }
  function handleKeyDown(event) {
    if (typeof onKeyDown === "function") onKeyDown(event);
    if (event.defaultPrevented) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setIsOpen(true);
    }
  }

  return (
    <button
      ref={anchorRef}
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-controls={listboxId}
      data-slot="select-trigger"
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      className={cx(
        "flex w-full items-center justify-between gap-2 rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
        className
      )}
      {...props}
    >
      {children}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4 shrink-0 opacity-50">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>
  );
}

export function SelectValue({ value, placeholder, className, ...props }) {
  const { value: currentValue } = useSelectContext("SelectValue");
  const shown = value !== undefined ? value : currentValue || undefined;
  return (
    <span className={cx("line-clamp-1 flex items-center gap-2", className)} {...props}>
      {shown || <span className="text-muted-foreground">{placeholder}</span>}
    </span>
  );
}

export function SelectContent({ className, children, ...props }) {
  const { isOpen, setIsOpen, anchorRef, panelRef, listboxId } = useSelectContext("SelectContent");
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, { side: "bottom", align: "start", offset: 4 });
  const width = anchorRef.current ? anchorRef.current.getBoundingClientRect().width : 0;

  useEffect(() => {
    if (!isOpen) return;
    const container = panelRef.current;
    if (!container) return;
    const selected = container.querySelector('[role="option"][aria-selected="true"]');
    const target = selected || container.querySelector(ITEM_SELECTOR);
    if (target) target.focus();
  }, [isOpen]);

  if (!isOpen) return null;

  function onKeyDown(event) {
    const container = panelRef.current;
    if (!container) return;
    const items = Array.from(container.querySelectorAll(ITEM_SELECTOR));
    if (items.length === 0) return;
    const index = items.indexOf(document.activeElement);
    if (event.key === "ArrowDown") {
      event.preventDefault();
      items[(index + 1 + items.length) % items.length].focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      items[(index - 1 + items.length) % items.length].focus();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (document.activeElement && document.activeElement.click) document.activeElement.click();
    } else if (event.key === "Escape") {
      setIsOpen(false);
    }
  }

  function onClick(event) {
    const target = event.target;
    if (target && target.closest && target.closest('[data-dismiss="true"]')) setIsOpen(false);
  }

  const node = (
    <div
      ref={panelRef}
      id={listboxId}
      role="listbox"
      className="fixed z-50 max-h-96 overflow-auto"
      style={{ top: `${top}px`, left: `${left}px`, width: `${width}px` }}
      onKeyDown={onKeyDown}
      onClick={onClick}
    >
      <div data-slot="select-content" className={cx("rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md", className)} {...props}>
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function SelectGroup({ className, children, ...props }) {
  return (
    <div role="group" data-slot="select-group" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export function SelectLabel({ className, children, ...props }) {
  return (
    <div data-slot="select-label" className={cx("px-2 py-1.5 text-xs text-muted-foreground", className)} {...props}>
      {children}
    </div>
  );
}

export function SelectItem({ value, selected, disabled, className, children, onSelect, ...props }) {
  const { value: currentValue, selectValue } = useSelectContext("SelectItem");
  const isSelected = selected !== undefined ? selected : currentValue === value;

  function handleClick() {
    if (disabled) return;
    selectValue(value);
    onSelect && onSelect(value);
  }

  return (
    <div
      role="option"
      tabIndex={-1}
      aria-selected={!!isSelected}
      aria-disabled={disabled || undefined}
      data-dismiss="true"
      data-slot="select-item"
      onClick={handleClick}
      className={cx(
        "relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm outline-none select-none focus:bg-accent focus:text-accent-foreground",
        disabled ? "pointer-events-none opacity-50" : "",
        isSelected ? "bg-accent text-accent-foreground" : "",
        className
      )}
      {...props}
    >
      {children}
      {isSelected ? (
        <span className="absolute right-2 flex size-3.5 items-center justify-center">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="size-4">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </span>
      ) : null}
    </div>
  );
}

export default Select;
