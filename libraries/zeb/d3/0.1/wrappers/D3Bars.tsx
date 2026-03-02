export const app = {};

export default function D3Bars(props) {
  return (
    <div
      data-zeb-lib="d3"
      data-config="{props.config}"
      className="w-full h-full"
      hydrate="visible"
    />
  );
}
