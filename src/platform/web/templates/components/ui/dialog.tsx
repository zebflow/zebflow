import { cx } from "zeb";
import { useState } from "zeb";

export function Dialog({ open, defaultOpen = false, onOpenChange, children }: any) {
  const [internal, setInternal] = useState(defaultOpen);
  const controlled = open !== undefined;
  const isOpen = controlled ? open : internal;

  const toggle = (v: boolean) => {
    if (!controlled) setInternal(v);
    onOpenChange?.(v);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onOpen: () => toggle(true), _onClose: () => toggle(false) } };
  });

  return (
    <>
      {enhanced}
      <span hidden tw-variants="max-w-lg p-6 gap-4 gap-3 gap-1.5 rounded-xl shadow-lg flex-col inset-0 z-50 items-center justify-center opacity-60 opacity-100 right-4 top-4 transition-opacity bg-black/80" />
    </>
  );
}

export default Dialog;
