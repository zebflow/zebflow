import { cx } from "zeb";

interface TextareaProps {
  placeholder?: string;
  value?: string;
  defaultValue?: string;
  onChange?: (e: any) => void;
  disabled?: boolean;
  rows?: number;
  className?: string;
  [key: string]: any;
}

export function Textarea({ className, ...rest }: TextareaProps) {
  return (
    <textarea
      className={cx(
        "flex min-h-[60px] w-full rounded-md border border-gray-200 bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-gray-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-gray-950 disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
      {...rest}
    />
  );
}

export default Textarea;
