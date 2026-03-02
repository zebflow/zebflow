export const app = {};

export default function VrmViewer(props) {
  return (
    <div
      data-zeb-lib="threejs-vrm"
      data-zeb-wrapper="VrmViewer"
      data-config="{{props.config}}"
      className="w-full h-full"
      hydrate="visible"
    ></div>
  );
}
