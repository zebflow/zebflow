import { cx } from "zeb/react";

interface ToastItem {
  id: number;
  msg: string;
  variant?: string;
}

interface SonnerProps {
  toasts?: ToastItem[];
  /** Render in place instead of fixed to the corner — for previews. */
  inline?: boolean;
}

const VARIANT_CLASSES = {
  info: "border-info/30 bg-popover text-popover-foreground",
  success: "border-success/40 bg-popover text-popover-foreground",
  warning: "border-warning/40 bg-popover text-popover-foreground",
  error: "border-destructive/40 bg-popover text-popover-foreground",
};
const DOT_CLASSES = {
  info: "bg-info",
  success: "bg-success",
  warning: "bg-warning",
  error: "bg-destructive",
};

/** A stack of short-lived notices, newest at the bottom. */
export default function Sonner(props: SonnerProps) {
  const toasts = Array.isArray(props?.toasts) ? props.toasts : [];
  return (
    <div
      className={cx("flex flex-col gap-2", props?.inline ? "" : "fixed bottom-4 right-4 z-[1100] w-80 max-w-[calc(100vw-2rem)]")}
      aria-live="polite"
      aria-atomic="true"
    >
      <span hidden tw-variants="border-info/30 border-success/40 border-warning/40 border-destructive/40 bg-info bg-success bg-warning bg-destructive" />
      {toasts.map((t) => {
        const variant = t.variant && VARIANT_CLASSES[t.variant] ? t.variant : "info";
        return (
          <div
            key={t.id}
            role="status"
            className={cx("flex items-start gap-2.5 rounded-lg border px-3.5 py-2.5 text-sm shadow-lg", VARIANT_CLASSES[variant])}
          >
            <span className={cx("mt-1.5 h-2 w-2 shrink-0 rounded-full", DOT_CLASSES[variant])} />
            <span className="min-w-0 flex-1 break-words">{t.msg}</span>
          </div>
        );
      })}
    </div>
  );
}
