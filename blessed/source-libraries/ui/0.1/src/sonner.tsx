import { cx, useState, useEffect } from "zeb/react";

/**
 * Toaster — shadcn's sonner wrapper, reimplemented from scratch: the engine
 * has no npm, so there is no `sonner` package underneath this one. A
 * `toast()` call and the mounted `Toaster` never share a parent, so they
 * talk through a module-level subscriber list instead of props or context —
 * `toast()` pushes into a shared queue and notifies every mounted `Toaster`;
 * `Toaster` subscribes to it in an effect, replaying whatever is already in
 * the queue at that moment (a `toast()` fired before the effect runs — from
 * another component's own mount effect, say — used to be lost until the
 * next push), and renders whatever the queue holds. Mount exactly one
 * `Toaster` per page (usually near the root) — mounting a second one just
 * means both render the same queue.
 *
 * `toast()` is a no-op when there is no `document` — the queue and its
 * timers are module-level state, so mutating them during SSR would leak
 * across requests in a reused V8 worker isolate. Error toasts announce with
 * `aria-live="assertive"`; every other type uses `polite`.
 */

let queue = [];
let nextId = 1;
let listeners = [];

function notify() {
  const snapshot = queue.slice();
  listeners.forEach((fn) => fn(snapshot));
}

function push(message, options) {
  if (typeof document === "undefined") return null;
  const opts = options || {};
  const item = {
    id: nextId++,
    type: opts.type || "default",
    title: typeof message === "string" ? message : opts.title,
    description: opts.description,
    duration: opts.duration == null ? 4000 : opts.duration,
  };
  queue = queue.concat([item]);
  notify();
  if (item.duration > 0) {
    setTimeout(() => dismiss(item.id), item.duration);
  }
  return item.id;
}

function dismiss(id) {
  queue = queue.filter((item) => item.id !== id);
  notify();
}

export function toast(message, options) {
  return push(message, options);
}
toast.success = (message, options) => push(message, { ...options, type: "success" });
toast.error = (message, options) => push(message, { ...options, type: "error" });
toast.warning = (message, options) => push(message, { ...options, type: "warning" });
toast.info = (message, options) => push(message, { ...options, type: "info" });
toast.dismiss = dismiss;

const TYPE_ACCENT = {
  default: "border-border",
  success: "border-success",
  error: "border-destructive",
  warning: "border-warning",
  info: "border-info",
};

const TYPE_ICON = {
  success: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4 shrink-0 text-success">
      <path d="M20 6 9 17l-5-5" />
    </svg>
  ),
  error: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4 shrink-0 text-destructive">
      <circle cx="12" cy="12" r="10" />
      <path d="m15 9-6 6M9 9l6 6" />
    </svg>
  ),
  warning: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4 shrink-0 text-warning">
      <path d="M12 9v4M12 17h.01" />
      <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" />
    </svg>
  ),
  info: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4 shrink-0 text-info">
      <circle cx="12" cy="12" r="10" />
      <path d="M12 16v-4M12 8h.01" />
    </svg>
  ),
};

export function Toaster({ className }) {
  const [toasts, setToasts] = useState([]);

  useEffect(() => {
    setToasts(queue.slice());
    listeners = listeners.concat([setToasts]);
    return () => {
      listeners = listeners.filter((fn) => fn !== setToasts);
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div data-slot="toaster" className={cx("fixed bottom-4 right-4 z-50 flex w-full max-w-sm flex-col gap-2", className)}>
      {toasts.map((item) => (
        <div
          key={item.id}
          role="status"
          aria-live={item.type === "error" ? "assertive" : "polite"}
          aria-atomic="true"
          data-slot="toast"
          className={cx(
            "flex items-start gap-3 rounded-lg border bg-popover p-4 text-sm text-popover-foreground shadow-lg",
            TYPE_ACCENT[item.type] || TYPE_ACCENT.default
          )}
        >
          {TYPE_ICON[item.type] || null}
          <div className="flex-1">
            {item.title ? <p className="font-medium">{item.title}</p> : null}
            {item.description ? <p className="text-muted-foreground">{item.description}</p> : null}
          </div>
          <button
            type="button"
            aria-label="Dismiss"
            onClick={() => dismiss(item.id)}
            className="shrink-0 text-muted-foreground hover:text-foreground"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      ))}
    </div>
  );
}

export default Toaster;
