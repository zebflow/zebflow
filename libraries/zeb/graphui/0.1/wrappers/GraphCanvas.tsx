export const app = {};

export default function GraphCanvas(props) {
  return (
    <div
      data-zeb-lib="graphui"
      data-zeb-wrapper="GraphCanvas"
      data-config="{{props.config}}"
      className="w-full h-full"
    ></div>
  );
}
