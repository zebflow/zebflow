import { cx } from "zeb";
import { useState, useRef } from "zeb";

interface InputOTPProps {
  maxLength?: number;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  onComplete?: (value: string) => void;
  disabled?: boolean;
  className?: string;
  children?: any;
  [key: string]: any;
}

interface InputOTPGroupProps {
  className?: string;
  children?: any;
  // injected
  _value?: string;
  _maxLength?: number;
  _onValueChange?: (v: string, idx: number) => void;
  _inputRefs?: any;
  [key: string]: any;
}

interface InputOTPSlotProps {
  index: number;
  className?: string;
  // injected
  _value?: string;
  _onValueChange?: (v: string, idx: number) => void;
  _inputRefs?: any;
  [key: string]: any;
}

export function InputOTP({ maxLength = 6, value, defaultValue = "", onChange, onComplete, disabled, className, children }: InputOTPProps) {
  const [internal, setInternal] = useState(defaultValue);
  const controlled = value !== undefined;
  const current = controlled ? value : internal;
  const inputRefs = useRef<HTMLInputElement[]>([]);

  const handleChange = (v: string, idx: number) => {
    const chars = current.split("");
    chars[idx] = v.slice(-1);
    const next = chars.join("").slice(0, maxLength);
    if (!controlled) setInternal(next);
    onChange?.(next);
    if (next.length === maxLength) onComplete?.(next);
    if (v && idx < maxLength - 1) inputRefs.current[idx + 1]?.focus();
  };

  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _value: current, _maxLength: maxLength, _onValueChange: handleChange, _inputRefs: inputRefs } };
  });

  return (
    <div className={cx("flex items-center gap-2", className)}>
      {enhanced}
    </div>
  );
}

export function InputOTPGroup({ className, children, _value, _onValueChange, _inputRefs, ...rest }: InputOTPGroupProps) {
  const items = Array.isArray(children) ? children : [children];
  const enhanced = items.map((child: any) => {
    if (!child || typeof child !== "object") return child;
    return { ...child, props: { ...child.props, _value, _onValueChange, _inputRefs } };
  });
  return (
    <div className={cx("flex items-center", className)} {...rest}>
      {enhanced}
    </div>
  );
}

export function InputOTPSlot({ index, className, _value, _onValueChange, _inputRefs, ...rest }: InputOTPSlotProps) {
  const char = _value?.[index] ?? "";
  const isActive = false; // simplified

  return (
    <div
      className={cx(
        "relative flex h-9 w-9 items-center justify-center border-y border-r border-gray-200 text-sm shadow-sm transition-all first:rounded-l-md first:border-l last:rounded-r-md",
        isActive ? "z-10 ring-1 ring-gray-950" : "",
        className
      )}
    >
      <input
        ref={(el) => { if (_inputRefs?.current && el) _inputRefs.current[index] = el; }}
        type="text"
        inputMode="numeric"
        maxLength={1}
        value={char}
        onChange={(e) => _onValueChange?.(e.target.value, index)}
        className="absolute inset-0 h-full w-full cursor-pointer rounded-[inherit] bg-transparent text-center text-sm caret-transparent focus:outline-none"
        aria-label={`OTP digit ${index + 1}`}
      />
      {char && <span className="pointer-events-none">{char}</span>}
    </div>
  );
}

export function InputOTPSeparator({ ...rest }: any) {
  return (
    <div role="separator" {...rest}>
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
        <path d="M5 12h14" />
      </svg>
    </div>
  );
}

export default InputOTP;
