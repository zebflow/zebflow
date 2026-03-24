// zeb/use 0.1 — RWE hooks collection
//
// Preact hooks are globals set by the RWE bootstrap before this bundle loads.
// Captured here once at module evaluation time.

const { useState, useEffect, useRef, useCallback } = globalThis;

// ── useDebounce ────────────────────────────────────────────────────────────
// Returns a debounced copy of `value` — updates only after `delay` ms idle.
export function useDebounce(value, delay = 150) {
  const [dv, setDv] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDv(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return dv;
}

// ── useThrottle ────────────────────────────────────────────────────────────
// Returns a throttled copy of `value` — updates at most once per `delay` ms.
export function useThrottle(value, delay = 150) {
  const [tv, setTv] = useState(value);
  const last = useRef(null);
  useEffect(() => {
    const now = Date.now();
    const since = last.current;
    if (since === null || now - since >= delay) {
      last.current = now;
      setTv(value);
    } else {
      const id = setTimeout(() => {
        last.current = Date.now();
        setTv(value);
      }, delay - (now - since));
      return () => clearTimeout(id);
    }
  }, [value, delay]);
  return tv;
}

// ── useLocalStorage ────────────────────────────────────────────────────────
// Persistent state backed by localStorage. JSON-serialised automatically.
// Returns [value, setter] — setter accepts value or updater function.
export function useLocalStorage(key, initialValue) {
  const [stored, setStored] = useState(() => {
    try {
      const item = window.localStorage.getItem(key);
      return item !== null ? JSON.parse(item) : initialValue;
    } catch {
      return initialValue;
    }
  });
  const setValue = useCallback((value) => {
    try {
      const next = typeof value === 'function' ? value(stored) : value;
      setStored(next);
      window.localStorage.setItem(key, JSON.stringify(next));
    } catch { /* ignore write errors */ }
  }, [key, stored]);
  return [stored, setValue];
}

// ── useClipboard ───────────────────────────────────────────────────────────
// Copy to clipboard with auto-reset. Returns { copied, copy }.
export function useClipboard(timeout = 2000) {
  const [copied, setCopied] = useState(false);
  const timer = useRef(null);
  const copy = useCallback((text) => {
    navigator.clipboard.writeText(String(text)).then(() => {
      setCopied(true);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), timeout);
    }).catch(() => {});
  }, [timeout]);
  useEffect(() => () => { if (timer.current) clearTimeout(timer.current); }, []);
  return { copied, copy };
}

// ── useTemporaryState ──────────────────────────────────────────────────────
// State that auto-resets to `initialValue` after `duration` ms.
// Returns [value, set].
export function useTemporaryState(initialValue, duration = 2000) {
  const [value, setValue] = useState(initialValue);
  const timer = useRef(null);
  const set = useCallback((newValue) => {
    setValue(newValue);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setValue(initialValue), duration);
  }, [initialValue, duration]);
  useEffect(() => () => { if (timer.current) clearTimeout(timer.current); }, []);
  return [value, set];
}

// ── useWindowEvent ─────────────────────────────────────────────────────────
// Attaches a window event listener with automatic cleanup on unmount.
// Handler ref is kept stable so you can pass inline functions safely.
export function useWindowEvent(event, handler, options) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    const fn = (e) => ref.current(e);
    window.addEventListener(event, fn, options);
    return () => window.removeEventListener(event, fn, options);
  }, [event]); // eslint-disable-line react-hooks/exhaustive-deps
}

// ── useLazyModule ──────────────────────────────────────────────────────────
// Loads a dynamic import once on mount. Returns [module, loading, error].
// Pass a loader: () => import('/path/to/module.mjs')
export function useLazyModule(loader) {
  const [mod, setMod] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  useEffect(() => {
    let cancelled = false;
    loader()
      .then((m) => { if (!cancelled) { setMod(m); setLoading(false); } })
      .catch((e) => { if (!cancelled) { setError(e); setLoading(false); } });
    return () => { cancelled = true; };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  return [mod, loading, error];
}

// ── useSearchParams ────────────────────────────────────────────────────────
// Reads and writes URL search params via history.replaceState (no reload).
// Returns [URLSearchParams, setter]. Setter accepts a string, URLSearchParams,
// or an updater function (prev: URLSearchParams) => string | URLSearchParams.
export function useSearchParams() {
  const [params, setParamsState] = useState(() => new URLSearchParams(window.location.search));
  const setParams = useCallback((updater) => {
    setParamsState((prev) => {
      const raw = typeof updater === 'function' ? updater(prev) : updater;
      const next = raw instanceof URLSearchParams ? raw : new URLSearchParams(raw);
      const qs = next.toString();
      window.history.replaceState(null, '', window.location.pathname + (qs ? '?' + qs : ''));
      return next;
    });
  }, []);
  return [params, setParams];
}

// ── useSplitPane ───────────────────────────────────────────────────────────
// Pointer-drag resizable split pane. Attach returned ref to the container.
// Requires a [data-split-handle] element inside. Sets a CSS custom property
// on the container to drive the panel width via CSS.
export function useSplitPane(options = {}) {
  const rootRef = useRef(null);
  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const handle = root.querySelector(options.handleSelector || '[data-split-handle]');
    if (!handle) return;
    const min = options.min ?? 220;
    const max = options.max ?? 420;
    const variable = options.variable ?? '--split-width';
    const startDrag = (event) => {
      event.preventDefault();
      const move = (e) => {
        const rect = root.getBoundingClientRect();
        root.style.setProperty(variable, `${Math.max(min, Math.min(max, e.clientX - rect.left))}px`);
      };
      const stop = () => {
        window.removeEventListener('pointermove', move);
        window.removeEventListener('pointerup', stop);
      };
      window.addEventListener('pointermove', move);
      window.addEventListener('pointerup', stop, { once: true });
    };
    handle.addEventListener('pointerdown', startDrag);
    return () => handle.removeEventListener('pointerdown', startDrag);
  }, [options.handleSelector, options.min, options.max, options.variable]);
  return rootRef;
}

// ── useClickAway ───────────────────────────────────────────────────────────
// Fires handler when a click or touch occurs outside the returned ref element.
export function useClickAway(handler) {
  const ref = useRef(null);
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  useEffect(() => {
    const fn = (e) => {
      if (ref.current && !ref.current.contains(e.target)) {
        handlerRef.current(e);
      }
    };
    document.addEventListener('mousedown', fn);
    document.addEventListener('touchstart', fn, { passive: true });
    return () => {
      document.removeEventListener('mousedown', fn);
      document.removeEventListener('touchstart', fn);
    };
  }, []);
  return ref;
}

// ── useInterval ────────────────────────────────────────────────────────────
// Runs `callback` on a fixed `delay` interval. Pass null to pause.
export function useInterval(callback, delay) {
  const cbRef = useRef(callback);
  cbRef.current = callback;
  useEffect(() => {
    if (delay == null) return;
    const id = setInterval(() => cbRef.current(), delay);
    return () => clearInterval(id);
  }, [delay]);
}

// ── useGeolocation ─────────────────────────────────────────────────────────
// Watches device position. Returns { loading, error, coords: { latitude, longitude, accuracy } }.
export function useGeolocation(options) {
  const [state, setState] = useState({ loading: true, error: null, coords: null });
  useEffect(() => {
    if (!navigator.geolocation) {
      setState({ loading: false, error: new Error('Geolocation not supported'), coords: null });
      return;
    }
    const onSuccess = (pos) => setState({
      loading: false, error: null,
      coords: { latitude: pos.coords.latitude, longitude: pos.coords.longitude, accuracy: pos.coords.accuracy },
    });
    const onError = (err) => setState({ loading: false, error: err, coords: null });
    const id = navigator.geolocation.watchPosition(onSuccess, onError, options);
    return () => navigator.geolocation.clearWatch(id);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  return state;
}

// ── useTree ────────────────────────────────────────────────────────────────
// Tracks expanded node IDs (strings) in a Set for tree UIs.
// Returns { expanded, isExpanded, toggle, expand, collapse, expandAll, collapseAll }.
export function useTree() {
  const [expanded, setExpanded] = useState(() => new Set());
  const isExpanded = useCallback((id) => expanded.has(id), [expanded]);
  const toggle = useCallback((id) => setExpanded((prev) => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  }), []);
  const expand = useCallback((id) => setExpanded((prev) => new Set([...prev, id])), []);
  const collapse = useCallback((id) => setExpanded((prev) => {
    const next = new Set(prev); next.delete(id); return next;
  }), []);
  const expandAll = useCallback((ids) => setExpanded(new Set(ids)), []);
  const collapseAll = useCallback(() => setExpanded(new Set()), []);
  return { expanded, isExpanded, toggle, expand, collapse, expandAll, collapseAll };
}
