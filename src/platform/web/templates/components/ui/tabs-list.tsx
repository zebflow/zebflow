export default function TabsList(props) {
  return (
    <div className={cx("inline-flex h-10 items-center justify-center rounded-md bg-[var(--zf-ui-bg-muted)] p-1 text-[var(--zf-ui-text-muted)]", props?.className)}>
      {props.children}
    </div>
  );
}
