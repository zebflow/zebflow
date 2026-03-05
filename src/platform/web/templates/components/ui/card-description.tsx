function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function CardDescription(props) {
  return (
    <p className={cx("text-sm text-slate-600 dark:text-slate-400", props?.className)}>
      <span>{props?.children ?? props?.label}</span>
    </p>
  );
}
