import { useRef, useState, useEffect, useLayoutEffect, useCallback } from "zeb/react";

/**
 * hooks — the small behaviours every overlay in zeb/ui is built from. There is
 * no Radix here: these hooks (plus two small non-hook helpers) are the whole
 * primitive layer. Every other zeb/ui file imports what it needs from
 * "zeb/ui/hooks" the same way it would import a component.
 *
 * `useClickAway` and `useEscape` listen in the capture phase so an overlay
 * closes before a click or key reaches whatever is underneath it.
 * `useEscape` and `useFocusTrap` arbitrate stacked overlays through a
 * module-level stack: only the top-most *active* instance of each ever acts,
 * so nesting a Popover inside a Dialog does not close both on one Escape.
 */

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [contenteditable]:not([contenteditable="false"]), [tabindex]:not([tabindex="-1"])';

function isFocusable(node) {
  if (!node) return false;
  if (node.hidden) return false;
  if (node.getAttribute("aria-hidden") === "true") return false;
  if (node.closest && node.closest("[inert]")) return false;
  if (typeof window !== "undefined" && window.getComputedStyle) {
    const style = window.getComputedStyle(node);
    if (style.display === "none" || style.visibility === "hidden") return false;
  }
  return true;
}

/** True for a zeb/react element vnode — used to detect "the caller already gave us an interactive child" without cloneElement. */
export function isVNode(value) {
  return !!value && typeof value === "object" && "type" in value;
}

/** Runs `ours` after `theirs` (a possibly-undefined prop handler), unless `theirs` called `event.preventDefault()`. */
export function composeEventHandlers(theirs, ours) {
  return (event) => {
    if (typeof theirs === "function") theirs(event);
    if (event && event.defaultPrevented) return;
    if (typeof ours === "function") ours(event);
  };
}

/** Calls `handler` when a pointer goes down (or a touch starts) outside every ref in `refs`, while `active`. */
export function useClickAway(refs, handler, active = true) {
  useEffect(() => {
    if (!active) return;
    const list = Array.isArray(refs) ? refs : [refs];
    function onPointerDown(event) {
      const inside = list.some((ref) => {
        const el = ref && ref.current;
        return el && el.contains(event.target);
      });
      if (inside) return;
      handler && handler(event);
    }
    document.addEventListener("mousedown", onPointerDown, true);
    document.addEventListener("touchstart", onPointerDown, true);
    return () => {
      document.removeEventListener("mousedown", onPointerDown, true);
      document.removeEventListener("touchstart", onPointerDown, true);
    };
  }, [refs, handler, active]);
}

let escapeStack = [];

/** Calls `handler` on Escape while `active` — but only for the top-most active caller, so stacked overlays dismiss one at a time. */
export function useEscape(active, handler) {
  const idRef = useRef(null);
  if (idRef.current === null) idRef.current = {};

  useEffect(() => {
    if (!active) return;
    const id = idRef.current;
    escapeStack = escapeStack.concat([id]);
    function onKeyDown(event) {
      if (event.key !== "Escape") return;
      if (escapeStack[escapeStack.length - 1] !== id) return;
      event.preventDefault();
      event.stopPropagation();
      handler && handler(event);
    }
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      escapeStack = escapeStack.filter((x) => x !== id);
    };
  }, [active, handler]);
}

let trapStack = [];

/**
 * Traps Tab/Shift+Tab inside `ref` while `active` — only for the top-most
 * active instance, so a popover opened from inside a dialog does not fight
 * the dialog for Tab. Focuses the first focusable element (or `ref` itself,
 * if there is none — callers give the container `tabIndex={-1}` for this) on
 * activation, and restores whatever had focus before it once `active` goes
 * back to false.
 */
export function useFocusTrap(ref, active) {
  const restoreRef = useRef(null);
  const idRef = useRef(null);
  if (idRef.current === null) idRef.current = {};

  useEffect(() => {
    if (!active) return;
    const el = ref && ref.current;
    if (!el) return;
    const id = idRef.current;
    trapStack = trapStack.concat([id]);
    restoreRef.current = document.activeElement;

    const focusables = () => Array.from(el.querySelectorAll(FOCUSABLE_SELECTOR)).filter(isFocusable);
    const first = focusables()[0];
    if (first) first.focus();
    else if (typeof el.focus === "function") el.focus();

    function onKeyDown(event) {
      if (event.key !== "Tab") return;
      if (trapStack[trapStack.length - 1] !== id) return;
      const items = focusables();
      if (items.length === 0) {
        event.preventDefault();
        el.focus();
        return;
      }
      const firstEl = items[0];
      const lastEl = items[items.length - 1];
      const activeIsInside = el.contains(document.activeElement) && items.indexOf(document.activeElement) !== -1;
      if (!activeIsInside) {
        event.preventDefault();
        (event.shiftKey ? lastEl : firstEl).focus();
        return;
      }
      if (event.shiftKey && document.activeElement === firstEl) {
        event.preventDefault();
        lastEl.focus();
      } else if (!event.shiftKey && document.activeElement === lastEl) {
        event.preventDefault();
        firstEl.focus();
      }
    }
    document.addEventListener("keydown", onKeyDown, true);

    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      trapStack = trapStack.filter((x) => x !== id);
      const toRestore = restoreRef.current;
      if (toRestore && typeof toRestore.focus === "function") toRestore.focus();
    };
  }, [ref, active]);
}

/**
 * The controlled/uncontrolled pattern shadcn's components all use: pass
 * `value` to run controlled, omit it (optionally with `defaultValue`) to let
 * the hook hold its own state. Returns `[value, set]` either way. `set`
 * accepts a plain value or a `(current) => next` updater; when uncontrolled,
 * the updater runs against real queued state (via `useState`'s own
 * functional-update form), so two updates in one tick compose instead of
 * clobbering each other.
 */
export function useControllable(value, defaultValue, onChange) {
  const [internal, setInternal] = useState(defaultValue);
  const isControlled = value !== undefined;

  const set = useCallback(
    (next) => {
      if (isControlled) {
        const resolved = typeof next === "function" ? next(value) : next;
        onChange && onChange(resolved);
        return;
      }
      let resolved;
      setInternal((current) => {
        resolved = typeof next === "function" ? next(current) : next;
        return resolved;
      });
      onChange && onChange(resolved);
    },
    [isControlled, value, onChange]
  );

  return [isControlled ? value : internal, set];
}

/**
 * Positions a floating panel against an anchor: fixed-coordinate `{top,
 * left}`, recomputed on resize/scroll and whenever the anchor or panel
 * change size (a `ResizeObserver` on both, not just window `resize`, so a
 * panel that grows after open — async content, fonts — repositions too).
 * Flips to the opposite side when the preferred side would overflow the
 * viewport.
 *
 * `options.watch` is an optional array of extra values to recompute on —
 * for an anchor that is not a real DOM node (`zeb/ui/context-menu`'s
 * pointer-anchor, whose `getBoundingClientRect()` reads wherever the click
 * landed) mutating a ref's `.current` does not by itself trigger this
 * effect, since the ref *object* passed in `anchorRef` never changes
 * identity; `watch` gives it something that does.
 */
export function useAnchoredPosition(anchorRef, panelRef, options) {
  const opts = options || {};
  const side = opts.side || "bottom";
  const align = opts.align || "center";
  const offset = opts.offset == null ? 8 : opts.offset;
  const watch = opts.watch || [];
  const [position, setPosition] = useState({ top: 0, left: 0 });
  // A panel usually mounts *after* the hook first ran (it renders only while
  // open), and a ref changing does not re-render anything. Notice the panel
  // element appearing or changing and run the placement again — otherwise
  // the first open sits at 0,0, the top-left of the page.
  const [mountTick, setMountTick] = useState(0);
  const seenPanel = useRef(null);
  useLayoutEffect(() => {
    const panel = panelRef && panelRef.current;
    if (panel !== seenPanel.current) {
      seenPanel.current = panel;
      setMountTick((n) => n + 1);
    }
  });

  useLayoutEffect(() => {
    const anchor = anchorRef && anchorRef.current;
    const panel = panelRef && panelRef.current;
    if (!anchor || !panel) return;

    function compute() {
      const a = anchor.getBoundingClientRect();
      const p = panel.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let resolvedSide = side;
      if (resolvedSide === "bottom" && a.bottom + offset + p.height > vh) resolvedSide = "top";
      else if (resolvedSide === "top" && a.top - offset - p.height < 0) resolvedSide = "bottom";
      else if (resolvedSide === "right" && a.right + offset + p.width > vw) resolvedSide = "left";
      else if (resolvedSide === "left" && a.left - offset - p.width < 0) resolvedSide = "right";

      let top = 0;
      let left = 0;
      if (resolvedSide === "top" || resolvedSide === "bottom") {
        top = resolvedSide === "bottom" ? a.bottom + offset : a.top - offset - p.height;
        if (align === "start") left = a.left;
        else if (align === "end") left = a.right - p.width;
        else left = a.left + a.width / 2 - p.width / 2;
      } else {
        left = resolvedSide === "right" ? a.right + offset : a.left - offset - p.width;
        if (align === "start") top = a.top;
        else if (align === "end") top = a.bottom - p.height;
        else top = a.top + a.height / 2 - p.height / 2;
      }

      left = Math.max(4, Math.min(left, vw - p.width - 4));
      top = Math.max(4, Math.min(top, vh - p.height - 4));
      setPosition({ top, left });
    }

    compute();
    window.addEventListener("resize", compute);
    window.addEventListener("scroll", compute, true);
    // Not every anchor is a real element — zeb/ui/context-menu hands this a
    // plain { getBoundingClientRect } object standing in for the pointer,
    // and ResizeObserver.observe() throws a TypeError on anything that
    // isn't an Element.
    let observer = null;
    if (typeof ResizeObserver !== "undefined") {
      observer = new ResizeObserver(compute);
      if (anchor instanceof Element) observer.observe(anchor);
      if (panel instanceof Element) observer.observe(panel);
    }
    return () => {
      window.removeEventListener("resize", compute);
      window.removeEventListener("scroll", compute, true);
      if (observer) observer.disconnect();
    };
  }, [anchorRef, panelRef, side, align, offset, mountTick, ...watch]);

  return position;
}

const hooks = { useClickAway, useEscape, useFocusTrap, useControllable, useAnchoredPosition, isVNode, composeEventHandlers };
export default hooks;
