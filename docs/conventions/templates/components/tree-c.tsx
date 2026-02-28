export const app = {};

export default function Page(input) {
  return (
<section className="tree-c">
  <h4>C Component</h4>
  <button onClick="tree.c.inc">Increment Shared</button>
  <button onClick="tree.c.dec">Decrement Shared</button>
  <p>Current: <span jText="shared.value">0</span></p>
  <p>Double (memo): <span jText="shared.double">0</span></p>
</section>
  );
}
