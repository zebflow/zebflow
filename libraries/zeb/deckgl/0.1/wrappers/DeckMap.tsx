export const app = {};

export default function DeckMap(props) {
  return (
    <div
      data-zeb-lib="deckgl"
      data-config="{props.config}"
      className="w-full h-full"
      hydrate="visible"
    />
  );
}
