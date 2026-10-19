// zeb/world's Zeb React component: Zebflow's own wrapper, not aslnut's code (MIT, Copyright (c) 2026 Zebflow; see
// the repository's LICENSE). Its interface is exactly aslnut's `aslnut/react` (<World options elements onReady
// viewChange headless className style>, useWorld, useElement, useWorldEvent), so aslnut's guide reads true here;
// build/interface.mjs fails the build when the two drift. Keeping a world in step with the props is aslnut's own
// `shell(w, { options })`: this file never adds, removes or restyles an element by itself.
//
// "zeb/react" is resolved by the build to build/zeb-react.mjs (Zeb React's global); its types, for the interface
// check, are React's.
import { createContext, createElement, useCallback, useContext, useEffect, useRef, useState, useSyncExternalStore } from "zeb/react";
import type { CSSProperties, ReactElement, ReactNode } from "zeb/react";
import { shell, world } from "@aslnut-src/index.ts";
import type { Element, Shell, ViewChange, World as AslnutWorld, WorldEvents, WorldOptions } from "@aslnut-src/index.ts";

export interface WorldProps {
  /** what the world is made with; later changes reach the options that have a setter (shell's rule) */
  options?: WorldOptions;
  /** aslnut's elements, each with an id: kept in step by id */
  elements?: readonly Element[];
  /** once per world, after its elements are added */
  onReady?: (w: AslnutWorld) => void;
  /** how a changed `options.view` moves the camera ('go', 'fly' or fly options); unset, it stays */
  viewChange?: ViewChange;
  /** no canvas, nothing drawn: world(null) */
  headless?: boolean;
  className?: string;
  /** the box the world fills; `position: relative` unless overridden */
  style?: CSSProperties;
  /** drawn over the world; they read it with the hooks */
  children?: ReactNode;
}

const WorldContext = createContext<AslnutWorld | null>(null);
const CANVAS_CSS = "display:block;width:100%;height:100%;touch-action:none";

/** a world on a canvas of its own: made on mount, disposed on unmount */
export function World(p: WorldProps): ReactElement {
  const box = useRef<HTMLDivElement>(null);
  const kept = useRef<Shell | null>(null);
  const [w, setW] = useState<AslnutWorld | null>(null);
  const headless = !!p.headless;

  // One world per mount. A world that resolves after unmount (strict mode, a fast navigation) is disposed at once.
  useEffect(() => {
    const options = p.options ?? {};
    const canvas = headless ? null : document.createElement("canvas");
    if (canvas) { canvas.style.cssText = CANVAS_CSS; box.current?.appendChild(canvas); }
    let mounted = true;
    let made: AslnutWorld | null = null;
    world(canvas, options).then(
      (x) => {
        if (!mounted) { x.dispose(); return; }
        made = x;
        kept.current = shell(x, { options });
        setW(x);
      },
      (e: unknown) => { if (mounted) (options.onError ?? console.error)(e); },
    );
    return () => {
      mounted = false;
      kept.current = null;
      made?.dispose();
      made = null;
      setW(null);
      canvas?.remove();
    };
  }, [headless]);

  // In this order, so the elements are in the world before onReady.
  useEffect(() => { if (w) kept.current?.elements(p.elements ?? []); }, [w, p.elements]);
  useEffect(() => { if (w) kept.current?.options(p.options ?? {}, p.viewChange); }, [w, p.options, p.viewChange]);
  useEffect(() => { if (w) p.onReady?.(w); }, [w]);

  return createElement(WorldContext.Provider, { value: w },
    createElement("div", { className: p.className, style: { position: "relative", ...p.style } },
      createElement("div", { ref: box, style: { position: "absolute", inset: 0 } }),
      p.children));
}

/** the world of the <World> around it: null until it is ready, and after it is gone */
export function useWorld(): AslnutWorld | null {
  return useContext(WorldContext);
}

/** an element of the world by id: null until it is added, and after it is removed. `w`: another world */
export function useElement<E extends Element = Element>(id: string, w?: AslnutWorld | null): E | null {
  const ctx = useContext(WorldContext);
  const target = w === undefined ? ctx : w;
  const subscribe = useCallback((changed: () => void) => {
    if (!target) return () => {};
    const off = target.on("elements", changed);
    return () => { off(); };
  }, [target]);
  const read = (): E | null =>
    target && !target.isDisposed ? ((target.list.find((x) => x.id === id) as E | undefined) ?? null) : null;
  return useSyncExternalStore(subscribe, read, () => null);
}

/** hear a world's event while mounted; the latest `fn` is called, subscribed once per world and name */
export function useWorldEvent<K extends keyof WorldEvents>(name: K, fn: (...e: WorldEvents[K]) => void, w?: AslnutWorld | null): void {
  const ctx = useContext(WorldContext);
  const target = w === undefined ? ctx : w;
  const latest = useRef(fn);
  useEffect(() => { latest.current = fn; });
  useEffect(() => {
    if (!target || target.isDisposed) return;
    const off = target.on(name, (...e: WorldEvents[K]) => latest.current(...e));
    return () => { off(); };
  }, [target, name]);
}

export type { ViewChange };
