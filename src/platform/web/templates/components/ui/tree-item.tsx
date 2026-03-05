function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function TreeItem(props) {
  const isFolder = Boolean(props?.isFolder);
  const isExpanded = props?.expanded !== false;
  const hasHref = Boolean(props?.href);

  return (
    <li
      className={cx(isFolder ? "project-tree-branch" : "project-tree-leaf", props?.className)}
      data-tree-item="true"
      data-id={props?.id ?? ""}
    >
      {isFolder ? (
        <details className="project-tree-details" open={isExpanded}>
          <summary className="project-tree-summary" onClick={props?.onClick}>
            <span className="project-tree-caret">
              <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
                <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            </span>
            {props?.icon ? <span className="project-tree-icon">{props.icon}</span> : null}
            <span className="project-tree-segment">{props?.label}</span>
          </summary>
          <ul className="project-tree-list">{props?.children}</ul>
        </details>
      ) : (
        <>
          {hasHref ? (
            <a
              href={props?.href}
              className={cx("project-tree-leaf-link", props?.active ? "is-active" : "")}
              onClick={props?.onClick}
            >
              {props?.icon ? <span className="project-tree-icon">{props.icon}</span> : null}
              <span className="project-tree-segment">{props?.label}</span>
              {props?.badge ? <span className="project-tree-meta">{props.badge}</span> : null}
            </a>
          ) : (
            <div
              className={cx("project-tree-leaf-link", props?.active ? "is-active" : "")}
              onClick={props?.onClick}
            >
              {props?.icon ? <span className="project-tree-icon">{props.icon}</span> : null}
              <span className="project-tree-segment">{props?.label}</span>
              {props?.badge ? <span className="project-tree-meta">{props.badge}</span> : null}
            </div>
          )}
        </>
      )}
    </li>
  );
}
