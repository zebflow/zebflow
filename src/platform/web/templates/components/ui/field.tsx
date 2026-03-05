import Label from "@/components/ui/label";

function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function Field(props) {
  return (
    <div className={cx("grid w-full items-center gap-2", props?.className)}>
      <Label label={props?.label} htmlFor={props?.id} />
      {props.children}
      {props?.description ? (
        <p className="text-[0.8rem] text-slate-500 dark:text-slate-400">
          <span>{props.description}</span>
        </p>
      ) : null}
    </div>
  );
}
