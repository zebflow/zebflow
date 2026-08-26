import { cx } from "zeb";
import { useState, useEffect } from "zeb";

interface ToastItem {
  id: number;
  title?: string;
  description?: string;
  variant?: "default" | "success" | "error" | "warning" | "info";
}

interface ToasterProps {
  position?: "top-left" | "top-center" | "top-right" | "bottom-left" | "bottom-center" | "bottom-right";
  className?: string;
}

const VARIANT_CLASSES = {
  default: "bg-white border-gray-200 text-gray-900",
  success: "bg-green-50 border-green-200 text-green-900",
  error:   "bg-red-50 border-red-200 text-red-900",
  warning: "bg-yellow-50 border-yellow-200 text-yellow-900",
  info:    "bg-blue-50 border-blue-200 text-blue-900",
};

const POSITION_CLASSES = {
  "top-left":      "top-4 left-4",
  "top-center":    "top-4 left-1/2 -translate-x-1/2",
  "top-right":     "top-4 right-4",
  "bottom-left":   "bottom-4 left-4",
  "bottom-center": "bottom-4 left-1/2 -translate-x-1/2",
  "bottom-right":  "bottom-4 right-4",
};

// Global toast queue
let _toastQueue: ToastItem[] = [];
let _nextId = 1;
let _listeners: Array<(toasts: ToastItem[]) => void> = [];

function notify() {
  _listeners.forEach(fn => fn([..._toastQueue]));
}

export const toast = {
  show(msg: string | { title?: string; description?: string }, variant: ToastItem["variant"] = "default") {
    const item: ToastItem = {
      id: _nextId++,
      variant,
      ...(typeof msg === "string" ? { title: msg } : msg),
    };
    _toastQueue = [..._toastQueue, item];
    notify();
    setTimeout(() => {
      _toastQueue = _toastQueue.filter(t => t.id !== item.id);
      notify();
    }, 4000);
  },
  success: (msg: string | { title?: string; description?: string }) => toast.show(msg, "success"),
  error:   (msg: string | { title?: string; description?: string }) => toast.show(msg, "error"),
  warning: (msg: string | { title?: string; description?: string }) => toast.show(msg, "warning"),
  info:    (msg: string | { title?: string; description?: string }) => toast.show(msg, "info"),
};

export function Toaster({ position = "bottom-right", className }: ToasterProps) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  useEffect(() => {
    _listeners.push(setToasts);
    return () => {
      _listeners = _listeners.filter(fn => fn !== setToasts);
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div
      aria-live="polite"
      aria-atomic="true"
      className={cx(
        "fixed z-[100] flex flex-col gap-2 pointer-events-none",
        POSITION_CLASSES[position] ?? POSITION_CLASSES["bottom-right"],
        className
      )}
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          role="status"
          className={cx(
            "pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-lg border p-4 shadow-lg",
            VARIANT_CLASSES[t.variant ?? "default"]
          )}
        >
          <div className="flex-1 space-y-1">
            {t.title && <p className="text-sm font-semibold">{t.title}</p>}
            {t.description && <p className="text-sm opacity-90">{t.description}</p>}
          </div>
          <button
            type="button"
            onClick={() => { _toastQueue = _toastQueue.filter(x => x.id !== t.id); notify(); }}
            className="shrink-0 opacity-50 hover:opacity-100"
            aria-label="Dismiss"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
              <path d="M18 6 6 18" /><path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      ))}
    </div>
  );
}

export default Toaster;
