export default function CardFooter(props) {
  return (
    <div className={cx("flex items-center p-6 pt-0", props?.className)}>
      {props.children}
    </div>
  );
}
