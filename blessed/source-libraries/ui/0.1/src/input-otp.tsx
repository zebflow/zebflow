import { cx, createContext, useContext, useRef, useState } from "zeb/react";

/**
 * InputOTP — a one-time-passcode field: `maxLength` single-character native
 * inputs that behave like one control. Composed through context exactly as
 * upstream composes its `OTPInputContext`: `InputOTPSlot` reads the joined
 * value, which slot is active, and the mutation callbacks from
 * `OtpContext` no matter how deep `InputOTPGroup` nests it. Controlled via
 * `value`, uncontrolled via `defaultValue`, like upstream. Differences from
 * upstream's `input-otp` library: each slot is its own visible
 * `<input>` (there is no single hidden full-width input with a fake caret),
 * so `data-active` reflects real DOM focus rather than a synthesized caret
 * position, and OS-level SMS autofill can only ever land on one slot — the
 * first, which carries `autocomplete="one-time-code"` as the best available
 * hook for it. A paste anywhere in the field is still distributed across
 * slots from that point on, and ArrowLeft/Right move between slots like
 * upstream.
 */

const OtpContext = createContext(null);

export function InputOTP({ maxLength = 6, value, defaultValue = "", onChange, onComplete, disabled, containerClassName, className, children, ...props }) {
  const [internal, setInternal] = useState(defaultValue);
  const isControlled = value !== undefined;
  const current = isControlled ? value : internal;
  const [activeIndex, setActiveIndex] = useState(-1);
  const inputsRef = useRef([]);

  function commit(next) {
    const trimmed = next.replace(/ +$/, "").slice(0, maxLength);
    if (!isControlled) setInternal(trimmed);
    onChange?.(trimmed);
    if (trimmed.length === maxLength) onComplete?.(trimmed);
    return trimmed;
  }

  function setChar(index, char) {
    const chars = current.padEnd(maxLength, " ").split("");
    chars[index] = char || " ";
    commit(chars.join(""));
  }

  // Distributes a pasted (or autofilled) string across slots starting at
  // `startIndex`; returns the index just past the last character written so
  // the caller can move focus there.
  function setChars(startIndex, str) {
    const chars = current.padEnd(maxLength, " ").split("");
    let i = startIndex;
    for (const ch of str) {
      if (i >= maxLength) break;
      chars[i] = ch;
      i++;
    }
    commit(chars.join(""));
    return i;
  }

  function focus(index) {
    const clamped = Math.max(0, Math.min(maxLength - 1, index));
    inputsRef.current[clamped]?.focus();
  }

  const ctx = {
    value: current,
    disabled,
    maxLength,
    activeIndex,
    setActiveIndex,
    setChar,
    setChars,
    focus,
    registerRef: (i, el) => {
      inputsRef.current[i] = el;
    },
  };

  return (
    <div data-slot="input-otp" className={cx("flex items-center gap-2", disabled ? "opacity-50" : "", containerClassName, className)} {...props}>
      <OtpContext.Provider value={ctx}>{children}</OtpContext.Provider>
    </div>
  );
}

export function InputOTPGroup({ className, children, ...props }) {
  return (
    <div data-slot="input-otp-group" className={cx("flex items-center", className)} {...props}>
      {children}
    </div>
  );
}

export function InputOTPSlot({ index, className, ...props }) {
  const otp = useContext(OtpContext);
  const char = (otp?.value?.[index] ?? "").trim();
  const isActive = otp?.activeIndex === index;

  return (
    <div
      data-slot="input-otp-slot"
      data-active={isActive ? "true" : "false"}
      className={cx(
        "relative flex h-9 w-9 items-center justify-center border-y border-r border-input text-sm shadow-xs outline-none first:rounded-l-md first:border-l last:rounded-r-md",
        isActive ? "z-10 border-ring ring-[3px] ring-ring/50" : "",
        className
      )}
    >
      <input
        ref={(el) => otp?.registerRef?.(index, el)}
        type="text"
        inputMode="numeric"
        autoComplete={index === 0 ? "one-time-code" : "off"}
        maxLength={1}
        value={char}
        disabled={otp?.disabled}
        onFocus={() => otp?.setActiveIndex?.(index)}
        onBlur={() => otp?.setActiveIndex?.(-1)}
        onInput={(e) => {
          // `onInput`, not `onChange`: on this engine `onChange` is the DOM
          // change event, which a text input only fires on blur.
          const next = e.target.value.slice(-1);
          otp?.setChar?.(index, next);
          if (next) otp?.focus?.(index + 1);
        }}
        onPaste={(e) => {
          const text = e.clipboardData?.getData("text") ?? "";
          const digits = text.replace(/\s/g, "");
          if (!digits) return;
          e.preventDefault();
          const nextIndex = otp?.setChars?.(index, digits) ?? index;
          otp?.focus?.(nextIndex);
        }}
        onKeyDown={(e) => {
          if (e.key === "Backspace" && !char) otp?.focus?.(index - 1);
          else if (e.key === "ArrowLeft") {
            e.preventDefault();
            otp?.focus?.(index - 1);
          } else if (e.key === "ArrowRight") {
            e.preventDefault();
            otp?.focus?.(index + 1);
          }
        }}
        className="absolute inset-0 h-full w-full cursor-text rounded-[inherit] bg-transparent text-center outline-none"
        aria-label={`Digit ${index + 1}`}
        {...props}
      />
    </div>
  );
}

export function InputOTPSeparator({ ...props }) {
  return (
    <div data-slot="input-otp-separator" role="separator" {...props}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
        <path d="M5 12h14" />
      </svg>
    </div>
  );
}

export default InputOTP;
