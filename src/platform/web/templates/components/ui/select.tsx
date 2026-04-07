import { useRef, useEffect, cx } from "zeb";

/**
 * Controlled <select> wrapper.
 *
 * Children must be <SelectOption> (or plain <option>) elements.
 * For SSR: renders `selected` on the matching <option> so browsers show
 * the correct value before hydration. After hydration, useEffect syncs
 * the DOM select.value for full React controlled-component behavior.
 *
 * Handles nested arrays from .map() inside children (Preact flattens
 * top-level arrays but .map() inside JSX can produce nested arrays).
 */
export function Select(props) {
  const ref = useRef<HTMLSelectElement>(null);
  const currentValue = String(props?.value ?? "");

  useEffect(() => {
    if (ref.current != null && props?.value != null) {
      ref.current.value = String(props.value);
    }
  }, [props?.value]);

  // Walk children (may be nested arrays from .map()), inject selected
  function withSelected(children: any): any {
    if (children == null) return null;
    if (Array.isArray(children)) return children.map(withSelected);
    if (typeof children === "object" && children.props) {
      const val = String(children.props.value ?? "");
      const isMatch = val === currentValue;
      if (Boolean(children.props.selected) !== isMatch) {
        // Return new JSX <option> — NOT a VNode spread
        return (
          <option
            key={children.key}
            value={children.props.value}
            selected={isMatch}
            disabled={children.props.disabled}
          >
            {children.props.label}
            {children.props.children}
          </option>
        );
      }
    }
    return children;
  }

  return (
    <div className={cx("relative group", props?.className)}>
      <select
        ref={ref}
        name={props?.name}
        required={Boolean(props?.required)}
        disabled={Boolean(props?.disabled)}
        value={props?.value}
        onChange={props?.onChange}
        className="flex h-9 w-full appearance-none rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-1 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 pr-8"
      >
        {withSelected(props.children)}
      </select>
      <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-ui-text-muted opacity-50">
        <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
          <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </div>
    </div>
  );
}

export function SelectOption(props) {
  return (
    <option value={props?.value} selected={Boolean(props?.selected)}>
      {props.label}
      {props.children}
    </option>
  );
}

export default Select;
