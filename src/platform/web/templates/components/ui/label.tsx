function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function Label(props) {
  return (
    <label
      htmlFor={props?.htmlFor ?? props?.for}
      className={cx("text-xs font-mono uppercase tracking-widest text-slate-500 leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70", props?.className)}
    >
      <span>{props.label}</span>
    </label>
  );
}
