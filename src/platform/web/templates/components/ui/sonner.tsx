interface ToastItem {
  id: number;
  msg: string;
  variant?: string;
}

interface SonnerProps {
  toasts?: ToastItem[];
}

export default function Sonner(props: SonnerProps) {
  const toasts = Array.isArray(props?.toasts) ? props.toasts : [];
  return (
    <div className="template-sonner" aria-live="polite" aria-atomic="true">
      {toasts.map((t) => (
        <div key={t.id} className={cx("template-toast", `is-${t.variant || "info"}`)}>{t.msg}</div>
      ))}
    </div>
  );
}
