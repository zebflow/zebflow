export default function NodeFieldSection({ field }) {
  return (
    <div className="flex items-center gap-3 pt-1">
      <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider whitespace-nowrap">
        {field.label}
      </span>
      <div className="flex-1 h-px bg-slate-200 dark:bg-slate-700" />
    </div>
  );
}
