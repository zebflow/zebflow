import { cx, createContext, useContext, useRef } from "zeb/react";

/**
 * Resizable — shadcn/ui's resizable panels, on Zebflow's engine.
 *
 * Upstream wraps `react-resizable-panels`; there is no such runtime library
 * here, so sizing is written by hand. `ResizablePanelGroup` hands its
 * `direction` down through a `ResizableContext` — matching how upstream's
 * `PanelGroup` shares it with `PanelResizeHandle` — so `ResizableHandle`
 * picks it up automatically; `direction` on the handle itself remains an
 * optional override for a caller that wants to force one. Sizing is real: on
 * pointer-down the handle reads its DOM `previousElementSibling`/
 * `nextElementSibling` (the two adjacent `ResizablePanel`s) and, on every
 * `pointermove`, writes `flexBasis` percentages on both directly as inline
 * styles — an imperative drag that bypasses re-renders, same as the
 * interactive part of the real thing, and SSR-safe since it only touches
 * `window`/DOM from inside the handler.
 */

const HANDLE_BAR = {
  horizontal: "h-full w-px cursor-col-resize",
  vertical: "w-full h-px cursor-row-resize",
};

const ResizableContext = createContext({ direction: "horizontal" });

export function ResizablePanelGroup({ direction = "horizontal", className, children, ...props }) {
  return (
    <ResizableContext.Provider value={{ direction }}>
      <div
        data-slot="resizable-panel-group"
        data-direction={direction}
        className={cx("flex h-full w-full", direction === "vertical" ? "flex-col" : "flex-row", className)}
        {...props}
      >
        {children}
      </div>
    </ResizableContext.Provider>
  );
}

export function ResizablePanel({ defaultSize = 50, minSize = 10, className, style, children, ...props }) {
  return (
    <div
      data-slot="resizable-panel"
      data-min-size={minSize}
      className={cx("overflow-auto", className)}
      style={{ flexBasis: `${defaultSize}%`, flexGrow: 0, flexShrink: 0, minWidth: 0, minHeight: 0, ...style }}
      {...props}
    >
      {children}
    </div>
  );
}

export function ResizableHandle({ direction: directionProp, withHandle, className, ...props }) {
  const ctx = useContext(ResizableContext);
  const direction = directionProp ?? ctx.direction;
  const ref = useRef(null);
  const onPointerDown = (e) => {
    const handle = ref.current;
    const before = handle?.previousElementSibling;
    const after = handle?.nextElementSibling;
    if (!handle || !before || !after) return;
    const vertical = direction === "vertical";
    const dim = (el) => (vertical ? el.getBoundingClientRect().height : el.getBoundingClientRect().width);
    const total = dim(handle.parentElement);
    const beforeStart = (dim(before) / total) * 100;
    const afterStart = (dim(after) / total) * 100;
    const originPos = vertical ? e.clientY : e.clientX;
    const minBefore = Number(before.getAttribute("data-min-size") ?? 10);
    const minAfter = Number(after.getAttribute("data-min-size") ?? 10);
    handle.setPointerCapture(e.pointerId);
    const onMove = (ev) => {
      const deltaPct = ((vertical ? ev.clientY : ev.clientX) - originPos) / total * 100;
      let nextBefore = beforeStart + deltaPct;
      let nextAfter = afterStart - deltaPct;
      if (nextBefore < minBefore) { nextAfter -= minBefore - nextBefore; nextBefore = minBefore; }
      if (nextAfter < minAfter) { nextBefore -= minAfter - nextAfter; nextAfter = minAfter; }
      before.style.flexBasis = `${nextBefore}%`;
      after.style.flexBasis = `${nextAfter}%`;
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  return (
    <div
      ref={ref}
      data-slot="resizable-handle"
      data-direction={direction}
      role="separator"
      aria-orientation={direction === "vertical" ? "horizontal" : "vertical"}
      tabIndex={0}
      onPointerDown={onPointerDown}
      className={cx(
        "relative flex shrink-0 items-center justify-center bg-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        direction === "vertical" ? HANDLE_BAR.vertical : HANDLE_BAR.horizontal,
        className
      )}
      {...props}
    >
      {withHandle ? (
        <div className={cx("z-10 flex h-4 w-3 items-center justify-center rounded-xs border border-border bg-border", direction === "vertical" ? "rotate-90" : "")}>
          <svg viewBox="0 0 10 16" className="size-2.5 text-muted-foreground" fill="currentColor">
            <circle cx="3" cy="2" r="1.1" />
            <circle cx="3" cy="8" r="1.1" />
            <circle cx="3" cy="14" r="1.1" />
            <circle cx="7" cy="2" r="1.1" />
            <circle cx="7" cy="8" r="1.1" />
            <circle cx="7" cy="14" r="1.1" />
          </svg>
        </div>
      ) : null}
    </div>
  );
}

export default ResizablePanelGroup;
