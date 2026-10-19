// "zeb/react" for src/react.ts, taken from Zeb React's global.
// The bundle is evaluated by the page preamble after Zeb React is installed, so the global is there.
const R = globalThis.__zebReact;
export const { createContext, createElement, useCallback, useContext, useEffect, useRef, useState, useSyncExternalStore } = R;
export default R;
