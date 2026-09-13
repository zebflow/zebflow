import { cx, createContext, useContext, useRef, useState, useEffect, createPortal } from "zeb/react";
import { useClickAway, useEscape, useAnchoredPosition } from "zeb/ui/hooks";

/**
 * ContextMenu — shadcn's context-menu, on Zebflow's engine. `ContextMenu`/
 * `ContextMenuTrigger`/`ContextMenuContent`/items compose through
 * `createContext` (no `child.type` detection): `ContextMenuTrigger` is the
 * element that listens for the browser's `contextmenu` event and doubles as
 * the click-away boundary; `ContextMenuContent` reads the click point off
 * context and anchors to it instead of to an element — done by handing
 * `useAnchoredPosition` a fake anchor object whose own
 * `getBoundingClientRect` returns a zero-size box at the click point, so it
 * gets the same viewport-flipping behaviour as every other anchored panel in
 * this library for free. That fake object is now stored directly as
 * `anchorRef.current` (not nested inside another `{ current }`, which made
 * `useAnchoredPosition` read `anchor.getBoundingClientRect` off `undefined`
 * and throw on the very first open), rebuilt fresh on every render so it
 * always closes over the latest point, and `useAnchoredPosition` is told to
 * `watch` that point — mutating `anchorRef.current` alone would not
 * otherwise re-trigger its position effect, so a second right-click while
 * the menu was already open used to keep the first click's coordinates.
 * `ContextMenuContent` portals to `document.body` in the browser and
 * renders inline during SSR.
 *
 * Same item/keyboard/`data-dismiss` behaviour as `zeb/ui/dropdown-menu`,
 * including checkbox items staying open and toggling instead of dismissing.
 * Dropped versus upstream: submenus, for the same reason `DropdownMenu`
 * drops them.
 */

const ITEM_SELECTOR = '[role^="menuitem"]:not([aria-disabled="true"])';

function pointerAnchor(point) {
  return {
    getBoundingClientRect: () => ({
      top: point.y,
      bottom: point.y,
      left: point.x,
      right: point.x,
      width: 0,
      height: 0,
    }),
  };
}

const ContextMenuContext = createContext(null);

function useContextMenuContext(name) {
  const ctx = useContext(ContextMenuContext);
  if (!ctx) throw new Error(`${name} must be rendered inside <ContextMenu>`);
  return ctx;
}

export function ContextMenu({ children }) {
  const [point, setPoint] = useState(null);
  const rootRef = useRef(null);
  const panelRef = useRef(null);

  useClickAway([rootRef, panelRef], () => setPoint(null), !!point);
  useEscape(!!point, () => setPoint(null));

  return <ContextMenuContext.Provider value={{ point, setPoint, rootRef, panelRef }}>{children}</ContextMenuContext.Provider>;
}

export function ContextMenuTrigger({ className, children, onContextMenu, ...props }) {
  const { setPoint, rootRef } = useContextMenuContext("ContextMenuTrigger");
  function handleContextMenu(event) {
    if (typeof onContextMenu === "function") onContextMenu(event);
    if (event.defaultPrevented) return;
    event.preventDefault();
    setPoint({ x: event.clientX, y: event.clientY });
  }
  return (
    <div ref={rootRef} data-slot="context-menu-trigger" className={cx("", className)} onContextMenu={handleContextMenu} {...props}>
      {children}
    </div>
  );
}

export function ContextMenuContent({ className, children, ...props }) {
  const { point, setPoint, panelRef } = useContextMenuContext("ContextMenuContent");
  const anchorRef = useRef(null);
  if (point) anchorRef.current = pointerAnchor(point);
  const { top, left } = useAnchoredPosition(anchorRef, panelRef, {
    side: "bottom",
    align: "start",
    offset: 2,
    watch: point ? [point.x, point.y] : [],
  });

  useEffect(() => {
    if (!point) return;
    const first = panelRef.current && panelRef.current.querySelector(ITEM_SELECTOR);
    if (first) first.focus();
  }, [point]);

  if (!point) return null;

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
    if (target && target.closest && target.closest('[data-dismiss="true"]')) setPoint(null);
  }

  const node = (
    <div ref={panelRef} role="menu" className="fixed z-50" style={{ top: `${top}px`, left: `${left}px` }} onKeyDown={onKeyDown} onClick={onClick}>
      <div
        data-slot="context-menu-content"
        className={cx("min-w-32 overflow-hidden rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md", className)}
        {...props}
      >
        {children}
      </div>
    </div>
  );

  return typeof document === "undefined" ? node : createPortal(node, document.body);
}

export function ContextMenuGroup({ className, children, ...props }) {
  return (
    <div role="group" data-slot="context-menu-group" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export function ContextMenuLabel({ className, inset, children, ...props }) {
  return (
    <div
      data-slot="context-menu-label"
      className={cx("px-2 py-1.5 text-sm font-medium text-foreground", inset ? "pl-8" : "", className)}
      {...props}
    >
      {children}
    </div>
  );
}

export function ContextMenuSeparator({ className, ...props }) {
  return <div role="separator" data-slot="context-menu-separator" className={cx("-mx-1 my-1 h-px bg-border", className)} {...props} />;
}

export function ContextMenuItem({ className, inset, variant = "default", disabled, children, ...props }) {
  return (
    <div
      role="menuitem"
      tabIndex={-1}
      aria-disabled={disabled || undefined}
      data-dismiss="true"
      data-slot="context-menu-item"
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

export function ContextMenuCheckboxItem({ className, checked, onCheckedChange, onClick, children, ...props }) {
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
      data-slot="context-menu-checkbox-item"
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

export function ContextMenuRadioGroup({ className, children, ...props }) {
  return (
    <div role="group" data-slot="context-menu-radio-group" className={cx("", className)} {...props}>
      {children}
    </div>
  );
}

export function ContextMenuRadioItem({ className, checked, children, ...props }) {
  return (
    <div
      role="menuitemradio"
      tabIndex={-1}
      aria-checked={!!checked}
      data-dismiss="true"
      data-slot="context-menu-radio-item"
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

export default ContextMenu;
