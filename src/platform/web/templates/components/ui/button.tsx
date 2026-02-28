export const app = {};

export default function Button(props) {
  return (
    <button
      type="{{props.type}}"
      className="inline-flex items-center justify-center font-bold uppercase tracking-widest transition-colors rounded-md @{props.variant|[bg-[#005B9A] text-white hover:bg-[#004A7A]]|[bg-white text-slate-900 border border-slate-300 hover:bg-slate-50]|[bg-transparent text-slate-700 hover:bg-slate-100]|[bg-slate-900 text-white hover:bg-slate-800]|default=[bg-[#005B9A] text-white hover:bg-[#004A7A]]} @{props.size|[px-3 py-2 text-[11px]]|[px-5 py-3 text-xs]|[px-6 py-4 text-sm]|default=[px-5 py-3 text-xs]} {{props.className}}"
    >
      <span>{props.label}</span>
    </button>
  );
}
