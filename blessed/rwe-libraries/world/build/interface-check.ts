// The interface guard (build/interface.mjs type-checks this file): Zebflow's wrapper must have exactly the
// interface of aslnut's `aslnut/react`. "@zeb-world/react" is ../src/react.ts (or a copy given to interface.mjs).
import type * as ours from "@zeb-world/react";
import type * as theirs from "@aslnut-src/core/world/host/react.ts";

type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

// Which props differ, by name: a drifting prop shows up as a property this empty object lacks.
type Prop<T, K> = K extends keyof T ? { type: T[K]; optional: {} extends Pick<T, K> ? true : false } : "missing";
type PropKeys = keyof ours.WorldProps | keyof theirs.WorldProps;
type PropDrift = {
  [K in PropKeys as Same<Prop<ours.WorldProps, K>, Prop<theirs.WorldProps, K>> extends true ? never : K]:
    { zebflow: Prop<ours.WorldProps, K>; aslnut: Prop<theirs.WorldProps, K> };
};
export const propDrift: PropDrift = {};

export const props: Same<ours.WorldProps, theirs.WorldProps> = true;
export const names: Same<keyof typeof ours, keyof typeof theirs> = true;
export const World: Same<typeof ours.World, typeof theirs.World> = true;
export const useWorld: Same<typeof ours.useWorld, typeof theirs.useWorld> = true;
export const useElement: Same<typeof ours.useElement, typeof theirs.useElement> = true;
export const useWorldEvent: Same<typeof ours.useWorldEvent, typeof theirs.useWorldEvent> = true;
export const viewChange: Same<ours.ViewChange, theirs.ViewChange> = true;
