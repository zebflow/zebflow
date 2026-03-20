import { usePageState } from 'zeb';

export const app = {};

export default function Page(input) {
  const state = usePageState();
  return (
<section className="tree-f">
  <h4>F Component</h4>
  <p>F reads shared value: <strong>{state.shared?.value ?? 0}</strong></p>
  <p>Last update: <span>{state.shared?.lastAction ?? 'init'}</span></p>
  <button onClick="tree.f.reset">Reset to Seed</button>
</section>
  );
}
