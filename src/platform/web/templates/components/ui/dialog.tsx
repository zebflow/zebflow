export default function Dialog(props) {
  return (
    <dialog
      id={props?.id}
      className={cx("backdrop:bg-slate-950/80 backdrop:backdrop-blur-sm p-0 rounded-lg border border-[var(--zf-ui-border)] bg-[var(--zf-ui-bg)] text-[var(--zf-ui-text)] shadow-lg overflow-hidden w-full max-w-lg", props?.className)}
      data-dialog={props?.id}
    >
      {props.children}
    </dialog>
  );
}
