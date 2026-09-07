// RWE runtime shim — re-exports framework globals installed by zeb_ssr_init.js
// and build_client_module() before any module loads.
// Allows: import { useState, useRouter, Link } from "zeb/react" in any file.
export const useState     = (globalThis as any).useState;
export const useEffect    = (globalThis as any).useEffect;
export const useRef       = (globalThis as any).useRef;
export const useMemo      = (globalThis as any).useMemo;
export const usePageState = (globalThis as any).usePageState;
export const useRouter    = (globalThis as any).useRouter;
export const Link         = (globalThis as any).Link;
export const cx           = (globalThis as any).cx;
