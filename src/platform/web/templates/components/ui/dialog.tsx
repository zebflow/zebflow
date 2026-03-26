import { useState } from "zeb";

/**
 * Dialog — controlled/uncontrolled open-state wrapper.
 *
 * Injects _isOpen / _onOpen / _onClose into its direct children
 * (typically a single <DialogContent>).
 *
 * Usage (controlled):
 *   <Dialog open={open} onOpenChange={setOpen}>
 *     <DialogContent>…</DialogContent>
 *   </Dialog>
 *
 * Usage (uncontrolled):
 *   <Dialog defaultOpen>
 *     <DialogContent>…</DialogContent>
 *   </Dialog>
 */
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
    return {
      ...child,
      props: {
        ...child.props,
        _isOpen: isOpen,
        _onOpen: () => toggle(true),
        _onClose: () => toggle(false),
      },
    };
  });

  return <>{enhanced}</>;
}

export default Dialog;
