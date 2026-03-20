export default function HelpTooltip({ text }) {
  return (
    <span className="zf-help-tooltip" tabIndex={0} aria-label={text}>
      <span className="zf-help-tooltip-icon" aria-hidden="true">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none">
          <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.8"/>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
          <circle cx="12" cy="17" r="0.5" fill="currentColor" stroke="currentColor" strokeWidth="1.5"/>
        </svg>
      </span>
      <span className="zf-help-tooltip-content" role="tooltip">{text}</span>
    </span>
  );
}
