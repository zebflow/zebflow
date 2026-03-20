import { cx } from "zeb";

/**
 * DropdownMenu — shadcn-style state-driven dropdown.
 *
 * Usage:
 *   <DropdownMenu trigger={<Button size="sm" variant="outline">+ New</Button>} align="right">
 *     <DropdownMenuItem label="Option" onClick={...} />
 *   </DropdownMenu>
 *
 * - Opens/closes via useState (no <details>/<summary> native browser toggle)
 * - Closes on outside mousedown via useEffect document listener
 * - Closes on any child click via bubbling (onClick on the content panel)
 */
export default function DropdownMenu({ trigger, align = "left", className, children }) {
  const [open, setOpen] = useState(false);
  const ref = useRef(null);

  useEffect(() => {
    if (!open) return;
    const close = (e) => {
      if (ref.current && !ref.current.contains(e.target)) setOpen(false);
    };
    document.addEventListener("mousedown", close, true);
    return () => document.removeEventListener("mousedown", close, true);
  }, [open]);

  return (
    <div ref={ref} className={cx("relative inline-block", className)}>
      <div onClick={(e) => { e.stopPropagation(); setOpen((o) => !o); }}>
        {trigger}
      </div>
      {open && (
        <div
          className={cx(
            "absolute z-50 top-full mt-1 min-w-[8rem] overflow-hidden rounded-md p-1 shadow-md",
            "border border-[var(--studio-border)] bg-[var(--studio-panel)] text-[var(--studio-text)]",
            align === "right" ? "right-0" : "left-0"
          )}
          onClick={() => setOpen(false)}
        >
          {children}
        </div>
      )}
    </div>
  );
}
