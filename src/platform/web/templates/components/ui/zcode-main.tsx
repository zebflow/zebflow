function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function ZCodeMain(props) {
  return (
    <main className={cx("flex-1 flex flex-col overflow-hidden relative", props?.className)}>
      {props.children}
    </main>
  );
}
