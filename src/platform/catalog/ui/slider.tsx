import { cx } from "zeb";

interface SliderProps {
  min?: number;
  max?: number;
  step?: number;
  value?: number;
  defaultValue?: number;
  onValueChange?: (value: number) => void;
  disabled?: boolean;
  className?: string;
  [key: string]: any;
}

export function Slider({ min = 0, max = 100, step = 1, value, defaultValue, onValueChange, disabled, className, ...rest }: SliderProps) {
  const handleChange = (e: any) => {
    onValueChange?.(Number(e.target.value));
  };

  const pct = ((( (value ?? defaultValue ?? min) - min) / (max - min)) * 100).toFixed(1);

  return (
    <div className={cx("relative flex w-full touch-none select-none items-center", className)}>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        defaultValue={defaultValue}
        onChange={handleChange}
        disabled={disabled}
        className="w-full h-2 appearance-none rounded-full bg-gray-200 accent-gray-900 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
        style={{ backgroundImage: `linear-gradient(to right, rgb(17,24,39) ${pct}%, transparent ${pct}%)` }}
        {...rest}
      />
    </div>
  );
}

export default Slider;
