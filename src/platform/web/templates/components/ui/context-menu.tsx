import { cx, useState } from "zeb/react";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";
import DropdownMenuSeparator from "@/components/ui/dropdown-menu-separator";

/**
 * A menu of actions for one thing.
 *
 * Two ways in, because a menu nobody can see is a menu nobody uses: a `⋯`
 * button the reader can point at, and right-click on the same row for people
 * who reach for that. Both open the same list at the pointer.
 *
 * Wrap the row and describe the actions:
 *
 *   <ContextMenu items={[
 *     { label: "New file", onSelect: () => ... },
 *     { separator: true },
 *     { label: "Delete", variant: "destructive", onSelect: () => ... },
 *   ]}>
 *     <button>…</button>
 *   </ContextMenu>
 *
 * An item may be `{ separator: true }`, and may carry `disabled`, `icon` or
 * `variant: "destructive"`. An empty or all-disabled list still opens, so the
 * reader learns that there is nothing to do here rather than wondering whether
 * the right-click registered.
 *
 * Dismissal is a full-screen backdrop rather than a listener on `document`:
 * template files do not touch the DOM, and a backdrop also swallows the click
 * that closes the menu so it does not select whatever was underneath.
 */
export default function ContextMenu({ items, children, trigger, className, disabled }) {
  const [at, setAt] = useState(null as { x: number; y: number } | null);

  const entries = Array.isArray(items) ? items : [];

  function open(event) {
    if (disabled || !entries.length) return;
    event.preventDefault();
    event.stopPropagation();
    // A left-click on the button and a right-click on the row both carry the
    // pointer, so the menu opens in the same place either way.
    setAt({ x: event.clientX, y: event.clientY });
  }

  // `contents` by default so wrapping changes no layout. A caller that wants
  // the trigger to sit inside its own row passes that row's classes instead,
  // and then the children and the `⋯` are siblings in the caller's flex box.
  return (
    <div className={className || "contents"} onContextMenu={open}>
      {children}
      {trigger === undefined ? null : (
        <button
          type="button"
          title="Actions"
          aria-label="Actions"
          data-context-menu-trigger="true"
          className={cx(
            "shrink-0 rounded px-1 text-muted-foreground hover:bg-accent hover:text-foreground",
            at ? "bg-accent text-foreground" : "",
          )}
          onClick={open}
        >
          {trigger || "\u22ef"}
        </button>
      )}
      {at ? (
        <>
          {/* Catches the dismissing click so it lands nowhere else. */}
          <div
            data-context-menu-backdrop="true"
            className="fixed inset-0 z-[999]"
            onClick={(event) => {
              event.stopPropagation();
              setAt(null);
            }}
            onContextMenu={(event) => {
              event.preventDefault();
              setAt(null);
            }}
          />
          <div
            role="menu"
            data-context-menu="true"
            className={cx(
              "fixed z-[1000] min-w-[10rem] overflow-hidden rounded-md p-1 shadow-md",
              "border border-border bg-card text-foreground",
            )}
            style={{ left: `${at.x}px`, top: `${at.y}px` }}
          >
            {entries.map((item, index) =>
              item?.separator ? (
                <DropdownMenuSeparator key={`sep-${index}`} />
              ) : (
                <DropdownMenuItem
                  key={`${item?.label ?? "item"}-${index}`}
                  label={item?.label}
                  icon={item?.icon}
                  variant={item?.variant}
                  className={item?.disabled ? "pointer-events-none opacity-40" : ""}
                  onClick={() => {
                    setAt(null);
                    if (!item?.disabled) item?.onSelect?.();
                  }}
                />
              ),
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
