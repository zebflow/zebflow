import { cx, createContext, useContext, useRef, useEffect, createPortal } from "zeb/react";
import { useControllable, useClickAway, useEscape, useAnchoredPosition, isVNode, composeEventHandlers } from "zeb/ui/hooks";

/**
 * DropdownMenu — shadcn's dropdown-menu, on Zebflow's engine.
 *
 * `DropdownMenu`/`DropdownMenuTrigger`/`DropdownMenuContent`/items compose
 * through `createContext` (no `child.type` detection). `DropdownMenuTrigger`
 * anchors and opens the panel — a real `<button>` for plain content, or an
 * `inline-flex` span around an element child (usually `<Button>`) so there
 * is no button-in-button — and carries `aria-haspopup="menu"` /
 * `aria-expanded`. `DropdownMenuContent` portals to `document.body` in the
 * browser (rendering inline during SSR) and positions against the trigger;
 * click-away checks both the trigger and the (portaled) panel and is gated
 * on `isOpen`.
 *
 * Arrow keys move real DOM focus between the panel's `[role="menuitem"]`
 * elements (read off the DOM, not React state — there is nothing to clone
 * item props into); Enter/Space activates whichever one has focus; Escape
 * and an outside click close the menu (`useEscape`'s stack means a nested
 * overlay's Escape does not also close this one). Tab closes the menu too,
 * but — unlike the rest — does not call `preventDefault`, so focus continues
 * to whatever the browser's normal tab order would land on next, matching
 * Radix rather than trapping focus inside an open menu. A click that
 * bubbles through any element carrying `data-dismiss` closes the menu,
 * except `DropdownMenuCheckboxItem` (a checkbox toggles and stays open, the
 * same as upstream) — `DropdownMenuRadioItem` keeps `data-dismiss`, since
 * selecting a radio item is upstream's cue to close.
 *
 * Dropped versus upstream: submenus (`DropdownMenuSub*`). A flyout submenu
 * needs its own hover-intent timer and anchored panel recursively, which is
 * disproportionate to what this gallery needs; nothing here stops one being
 * added the same way `Popover` was built.
 */

const ITEM_SELECTOR = '[role^="menuitem"]:not([aria-disabled="true"])';

const DropdownMenuContext = createContext(null);

function useDropdownMenuContext(name) {
  const ctx = useContext(DropdownMenuContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <DropdownMenu>`);
  return ctx;
}

export function DropdownMenu({ open, defaultOpen = false, onOpenChange, children }) {
  const [isOpen, setIsOpen] = useControllable(open, defaultOpen, onOpenChange);
  const anchorRef = useRef(null);
  const panelRef = useRef(null);

  useClickAway([anchorRef, panelRef], () => setIsOpen(false), isOpen);
  useEscape(isOpen, () => setIsOpen(false));

  return <DropdownMenuContext.Provider value={{ isOpen, setIsOpen, anchorRef, panelRef }}>{children}</DropdownMenuContext.Provider>;
}

export function DropdownMenuTrigger({ className, children, onClick, ...props }) {
  const { isOpen, setIsOpen, anchorRef } = useDropdownMenuContext("DropdownMenuTrigger");
  const handleClick = composeEventHandlers(onClick, () => setIsOpen(!isOpen));
  if (isVNode(children)) {
    return (
      <span
        ref={anchorRef}
        data-slot="dropdown-menu-trigger"
        className={cx("inline-flex", className)}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        onClick={handleClick}
        {...props}
      >
        {children}
      </span>
    );
  }
  return (
    <button
      ref={anchorRef}
      type="button"
      data-slot="dropdown-menu-trigger"
      className={cx("", className)}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      onClick={handleClick}
      {...props}
    >
      {children}
    </button>
  );
}

export function DropdownMenuContent({ className, children, ...props }) {
  const { isOpen, setIsOpen, anchorRef, panelRef } = useDropdownMenuContext("DropdownMenuContent");
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, { side: "bottom", align: "start", offset: 4 });

  useEffect(() => {
    if (!isOpen) return;
    const first = panelRef.current && panelRef.current.querySelector(ITEM_SELECTOR);
    if (first) first.focus();
  }, [isOpen]);

  if (!isOpen) return null;

  function onKeyDown(event) {
    const container = panelRef.current;
    if (!container) return;
    const items = Array.from(container.querySelectorAll(ITEM_SELECTOR));
    if (event.key === "Tab") {
      setIsOpen(false);
      return;
    }
    if (items.length === 0) return;
    const index = items.indexOf(document.activeElement);
    if (event.key === "ArrowDown") {
      event.preventDefault();
      items[(index + 1 + items.length) % items.length].focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      items[(index - 1 + items.length) % items.length].focus();
    } else if ((event.key === "Enter" || event.key === " ") && !event.defaultPrevented) {
      // Skip if the focused item already handled this key itself (a
      // checkbox item's own onKeyDown calls preventDefault before this
      // bubbles here) — bridging to a synthetic click on top of that would
      // fire the item's activation twice, toggling it and back.
      event.preventDefault();
      if (document.activeElement && document.activeElement.click) document.activeElement.click();
    }
  }

  function onClick(event) {
    const target = event.target;
    if (target && target.closest && target.closest('[data-dismiss="true"]')) setIsOpen(false);
  }

  const node = (
    <div ref={panelRef} role="menu" className="fixed z-50" style={{ top: `${top}px`, left: `${left}px` }} onKeyDown={onKeyDown} onClick={onClick}>
      <div
        data-slot="dropdown-menu-content"
        className={cx("min-w-32 overflow-hidden rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md", className)}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function DropdownMenuGroup({ className, children, ...props }) {
  return (
    <div role="group" data-slot="dropdown-menu-group" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export function DropdownMenuLabel({ className, inset, children, ...props }) {
  return (
    <div
      data-slot="dropdown-menu-label"
      className={cx("px-2 py-1.5 text-sm font-medium text-foreground", inset ? "pl-8" : "", className)}
      {...props}
    >
      {children}
    </div>
  );
}

export function DropdownMenuSeparator({ className, ...props }) {
  return <div role="separator" data-slot="dropdown-menu-separator" className={cx("-mx-1 my-1 h-px bg-border", className)} {...props} />;
}

export function DropdownMenuShortcut({ className, children, ...props }) {
  return (
    <span data-slot="dropdown-menu-shortcut" className={cx("ml-auto text-xs tracking-widest text-muted-foreground", className)} {...props}>
      {children}
    </span>
  );
}

export function DropdownMenuItem({ className, inset, variant = "default", disabled, children, ...props }) {
  return (
    <div
      role="menuitem"
      tabIndex={-1}
      aria-disabled={disabled || undefined}
      data-dismiss="true"
      data-slot="dropdown-menu-item"
      className={cx(
        "relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none select-none focus:bg-accent focus:text-accent-foreground",
        inset ? "pl-8" : "",
        disabled ? "pointer-events-none opacity-50" : "",
        variant === "destructive" ? "text-destructive focus:bg-destructive/10 focus:text-destructive" : "",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}

export function DropdownMenuCheckboxItem({ className, checked, onCheckedChange, onClick, children, ...props }) {
  function toggle() {
    onCheckedChange && onCheckedChange(!checked);
  }
  function handleClick(event) {
    if (typeof onClick === "function") onClick(event);
    if (!event.defaultPrevented) toggle();
  }
  function handleKeyDown(event) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      toggle();
    }
  }
  return (
    <div
      role="menuitemcheckbox"
      tabIndex={-1}
      aria-checked={!!checked}
      data-slot="dropdown-menu-checkbox-item"
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      className={cx(
        "relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-none select-none focus:bg-accent focus:text-accent-foreground",
        className
      )}
      {...props}
    >
      <span className="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        {checked ? (
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="size-4">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        ) : null}
      </span>
      {children}
    </div>
  );
}

export function DropdownMenuRadioGroup({ className, children, ...props }) {
  return (
    <div role="group" data-slot="dropdown-menu-radio-group" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export function DropdownMenuRadioItem({ className, checked, children, ...props }) {
  return (
    <div
      role="menuitemradio"
      tabIndex={-1}
      aria-checked={!!checked}
      data-dismiss="true"
      data-slot="dropdown-menu-radio-item"
      className={cx(
        "relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-none select-none focus:bg-accent focus:text-accent-foreground",
        className
      )}
      {...props}
    >
      <span className="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        {checked ? <span className="size-2 rounded-full bg-foreground" /> : null}
      </span>
      {children}
    </div>
  );
}

export default DropdownMenu;
