export default function DialogTitle(props) {
  return (
    <h3 className={cx("font-display text-[0.9rem] font-bold leading-[1.2] text-body", props?.className)}>
      {props.children}
      <span>{props.label}</span>
    </h3>
  );
}
