export default function TreeView(props) {
  return (
    <ul 
      className={cx("project-tree-root", props?.className)} 
      data-tree-view="true"
      data-hook={props?.hook ?? ""}
      data-template-tree={props?.isTemplateTree ? "true" : "false"}
      data-template-root-drop={props?.isTemplateRootDrop ? "true" : "false"}
    >
      {props.children}
    </ul>
  );
}
