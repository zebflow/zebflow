import { cx } from "zeb";

export default function CardHeader(props) {
  return (
    <div className={cx("flex flex-col space-y-1.5 px-6 py-4 border-b border-ui-border-subtle bg-ui-bg-subtle", props?.className)}>
      {props.children}
    </div>
  );
}
