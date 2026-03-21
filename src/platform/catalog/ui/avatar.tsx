import { cx } from "zeb";
import { useState } from "zeb";

interface AvatarProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

interface AvatarImageProps {
  src?: string;
  alt?: string;
  className?: string;
  [key: string]: any;
}

interface AvatarFallbackProps {
  className?: string;
  children?: any;
  [key: string]: any;
}

export function Avatar({ className, children, ...rest }: AvatarProps) {
  return (
    <span
      className={cx(
        "relative flex h-10 w-10 shrink-0 overflow-hidden rounded-full",
        className
      )}
      {...rest}
    >
      {children}
    </span>
  );
}

export function AvatarImage({ src, alt = "", className, ...rest }: AvatarImageProps) {
  const [errored, setErrored] = useState(false);
  if (errored) return null;
  return (
    <img
      src={src}
      alt={alt}
      onError={() => setErrored(true)}
      className={cx("aspect-square h-full w-full object-cover", className)}
      {...rest}
    />
  );
}

export function AvatarFallback({ className, children, ...rest }: AvatarFallbackProps) {
  return (
    <span
      className={cx(
        "flex h-full w-full items-center justify-center rounded-full bg-gray-100 text-gray-900 text-sm font-medium",
        className
      )}
      {...rest}
    >
      {children}
    </span>
  );
}

export default Avatar;
