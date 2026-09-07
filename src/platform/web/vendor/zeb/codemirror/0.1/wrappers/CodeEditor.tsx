export const app = {};

export default function CodeEditor(props) {
  return (
    <div
      data-zeb-lib="codemirror"
      data-zeb-wrapper="CodeEditor"
      data-config="{{props.config}}"
      className="w-full h-full"
    ></div>
  );
}
