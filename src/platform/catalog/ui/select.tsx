import { cx } from "zeb";
import { useState, useRef } from "zeb";
import { useClickAway } from "zeb/use";

interface SelectProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  disabled?: boolean;
  children?: any;
  [key: string]: any;
}

export function Select({ value, defaultValue, onValueChange, disabled, children }: SelectProps) {
  const [internal, setInternal] = useState(defaultValue ?? "");
  const [open, setOpen] = useState(false);
  const controlled = value !== undefined;
  const current = controlled ? value : internal;

  const containerRef = useRef<HTMLDivElement>(null);
  useClickAway(containerRef, () => setOpen(false));

  const handleSelect = (v: string) => {
    if (!controlled) setInternal(v);
    onValueChange?.(v);
    setOpen(false);
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _current: current, _open: open, _onOpen: () => !disabled && setOpen(!open), _onSelect: handleSelect } };
  });

  return (
    <div ref={containerRef} className="relative">
      {enhanced}
      <span hidden tw-variants="absolute z-50 max-h-96 min-w-[8rem] w-full overflow-auto rounded-md border border-gray-200 bg-white text-gray-900 shadow-md mt-1 p-1 relative flex cursor-pointer select-none items-center rounded-sm py-1.5 pl-8 pr-2 outline-none pointer-events-none opacity-50 hover:bg-gray-100 bg-gray-100 font-medium left-2 h-3.5 w-3.5" />
    </div>
  );
}

export function SelectTrigger({ className, children, _open, _onOpen, ...rest }: any) {
  return (
    <button
      type="button"
      role="combobox"
      aria-expanded={_open}
      onClick={_onOpen}
      className={cx(
        "flex h-9 w-full items-center justify-between whitespace-nowrap rounded-md border border-gray-200 bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-white placeholder:text-gray-500 focus:outline-none focus:ring-1 focus:ring-gray-950 disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1",
        className
      )}
      {...rest}
    >
      {children}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4 opacity-50 shrink-0">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>
  );
}

export function SelectValue({ placeholder, _current, className, ...rest }: any) {
  return (
    <span className={cx("", className)} {...rest}>
      {_current || <span className="text-gray-500">{placeholder}</span>}
    </span>
  );
}

export function SelectContent({ className, children, _open, _onSelect, _current, ...rest }: any) {
  if (!_open) return null;

  const inject = (child: any): any => {
    if (!child || typeof child !== "object") return child;
    const props = child.props ?? {};
    const newProps: any = { ...props, _onSelect, _current };
    if (Array.isArray(props.children)) newProps.children = props.children.map(inject);
    else if (props.children && typeof props.children === "object") newProps.children = inject(props.children);
    return { ...child, props: newProps };
  };

  const items = Array.isArray(children) ? children : [children];
  return (
    <div
      role="listbox"
      className={cx(
        "absolute z-50 max-h-96 min-w-[8rem] w-full overflow-auto rounded-md border border-gray-200 bg-white text-gray-900 shadow-md mt-1",
        className
      )}
      {...rest}
    >
      <div className="p-1">
        {items.map(inject)}
      </div>
    </div>
  );
}

export function SelectItem({ value, disabled, className, children, _onSelect, _current, ...rest }: any) {
  const selected = _current === value;
  return (
    <div
      role="option"
      aria-selected={selected}
      onClick={() => !disabled && _onSelect?.(value)}
      className={cx(
        "relative flex w-full cursor-pointer select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none",
        disabled ? "pointer-events-none opacity-50" : "hover:bg-gray-100",
        selected ? "bg-gray-100 font-medium" : "",
        className
      )}
      {...rest}
    >
      <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        {selected && (
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        )}
      </span>
      {children}
    </div>
  );
}

export function SelectGroup({ className, children, _onSelect, _current, ...rest }: any) {
  const inject = (child: any): any => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _onSelect, _current } };
  };
  const items = Array.isArray(children) ? children : [children];
  return (
    <div className={cx("", className)} {...rest}>
      {items.map(inject)}
    </div>
  );
}

export function SelectLabel({ className, children, ...rest }: any) {
  return (
    <div className={cx("px-2 py-1.5 text-sm font-semibold text-gray-500", className)} {...rest}>
      {children}
    </div>
  );
}

export function SelectSeparator({ className, ...rest }: any) {
  return <div className={cx("-mx-1 my-1 h-px bg-gray-100", className)} {...rest} />;
}

export default Select;
