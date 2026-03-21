import { cx } from "zeb";
import { useState, useRef } from "zeb";

interface HoverCardProps {
  openDelay?: number;
  closeDelay?: number;
  children?: any;
  [key: string]: any;
}

export function HoverCard({ openDelay = 700, closeDelay = 300, children }: HoverCardProps) {
  const [isOpen, setIsOpen] = useState(false);
  const openTimer = useRef<any>(null);
  const closeTimer = useRef<any>(null);

  const handleOpen = () => {
    clearTimeout(closeTimer.current);
    openTimer.current = setTimeout(() => setIsOpen(true), openDelay);
  };

  const handleClose = () => {
    clearTimeout(openTimer.current);
    closeTimer.current = setTimeout(() => setIsOpen(false), closeDelay);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _isOpen: isOpen, _onMouseEnter: handleOpen, _onMouseLeave: handleClose } };
  });

  return <>{enhanced}</>;
}

export function HoverCardTrigger({ children, _onMouseEnter, _onMouseLeave, asChild, className, ...rest }: any) {
  if (asChild && children && typeof children === "object") {
    return { ...children, props: { ...children.props, onMouseEnter: _onMouseEnter, onMouseLeave: _onMouseLeave } };
  }
  return (
    <span onMouseEnter={_onMouseEnter} onMouseLeave={_onMouseLeave} className={cx("inline-block", className)} {...rest}>
      {children}
    </span>
  );
}

export function HoverCardContent({ className, children, _isOpen, _onMouseEnter, _onMouseLeave, align = "center", sideOffset = 4, ...rest }: any) {
  if (!_isOpen) return null;
  return (
    <div
      role="tooltip"
      onMouseEnter={_onMouseEnter}
      onMouseLeave={_onMouseLeave}
      className={cx(
        "absolute z-50 w-64 rounded-md border border-gray-200 bg-white p-4 shadow-md outline-none",
        className
      )}
      style={{ top: "100%", marginTop: `${sideOffset}px`, left: align === "start" ? 0 : align === "end" ? "auto" : "50%", right: align === "end" ? 0 : "auto", transform: align === "center" ? "translateX(-50%)" : "none" }}
      {...rest}
    >
      {children}
    </div>
  );
}

export default HoverCard;
