export default function HelpTooltip({ text }) {
  return (
    <span
      className="group relative inline-flex items-center outline-none"
      tabIndex={0}
      aria-label={text}
    >
      <span
        className="inline-flex text-ui-text-muted transition-colors duration-150 group-hover:text-ui-text group-focus-within:text-ui-text"
        aria-hidden="true"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none">
          <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.8"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
          <circle cx="12" cy="17" r="0.5" fill="currentColor" stroke="currentColor" strokeWidth="1.5"/>
        </svg>
      </span>
      <span
        className="pointer-events-none invisible absolute bottom-[calc(100%+8px)] left-1/2 z-50 min-w-40 max-w-60 -translate-x-1/2 rounded-md border border-ui-border bg-gray-800 px-2.5 py-1.5 text-[11px] font-normal leading-[1.5] tracking-normal text-gray-100 opacity-0 shadow-lg transition-[opacity,visibility] duration-150 group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100"
        role="tooltip"
      >
        {text}
        <span className="absolute left-1/2 top-full h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[5px] border-x-transparent border-t-gray-800" />
      </span>
    </span>
  );
}
