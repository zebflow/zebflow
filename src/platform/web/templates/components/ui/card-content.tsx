import { cx } from "zeb";

export default function CardContent(props) {
  return (
    <div className={cx("px-6 py-4", props?.className)}>
      {props.children}
    </div>
  );
}
