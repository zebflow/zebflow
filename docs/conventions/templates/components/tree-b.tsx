export const app = {};

export default function Page(input) {
  return (
<section className="tree-b">
  <h3>B Component</h3>
  <p>B sees shared value: <strong zText="shared.value">0</strong></p>
  <TreeC />
</section>
  );
}
