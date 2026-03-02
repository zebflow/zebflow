export const app = {};

export default function ThreeScene(props) {
  return (
    <div
      data-zeb-lib="threejs"
      data-zeb-wrapper="ThreeScene"
      data-config="{{props.config}}"
      className="w-full h-full"
      hydrate="visible"
    ></div>
  );
}
