export default function Tabs(props) {
  return (
    <div className={cx("flex flex-col gap-4", props?.className)}>
      {props.children}
    </div>
  );
}
