import { cx } from "rwe";

export default function CardContent(props) {
  return (
    <div className={cx("px-7 py-6", props?.className)}>
      {props.children}
    </div>
  );
}
