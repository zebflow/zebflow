import { usePageState } from 'zeb';

export const app = {};

export default function Page(input) {
  const state = usePageState();
  return (
<section className="tree-b">
  <h3>B Component</h3>
  <p>B sees shared value: <strong>{state.shared?.value ?? 0}</strong></p>
  <TreeC />
</section>
  );
}
