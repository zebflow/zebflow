export default function Dialog(props) {
  return (
    <dialog
      id={props?.id}
      className={cx("backdrop:bg-slate-950/80 backdrop:backdrop-blur-sm p-0 rounded-lg border border-slate-200 bg-white shadow-lg dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50 overflow-hidden w-full max-w-lg", props?.className)}
      data-dialog={props?.id}
    >
      {props.children}
    </dialog>
  );
}
