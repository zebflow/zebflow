import { h, Fragment, render, hydrate, renderToString, useState, useReducer, useEffect,
  useLayoutEffect, useRef, useMemo, useCallback, useContext, useId, createContext,
  memo, forwardRef, useImperativeHandle, createPortal } from '../../../src/rwe/runtime/zeb_react.mjs';

const results = [];
const tick = () => new Promise(resolve => setTimeout(resolve, 20));
function assert(value, message) { if (!value) throw new Error(message); }
function equal(a, b, message) { assert(JSON.stringify(a) === JSON.stringify(b), message + ': ' + JSON.stringify(a)); }
async function test(name, fn) {
  const host = document.createElement('div'); document.body.append(host);
  try { await fn(host); results.push({ name, passed: true }); }
  catch (error) { results.push({ name, passed: false, error: String(error.stack) }); }
  finally { render(null, host); host.remove(); }
}

await test('hydration preserves DOM, early input, adjacent text, IDs and event handlers', async host => {
  let update;
  function App() {
    const [n, set] = useState(0); update = set;
    const id = useId();
    return h('section', null, h('input', { id, value: 'server', onInput: () => set(n => n + 1) }),
      h('button', { onClick: () => set(n => n + 1) }, 'count:', n));
  }
  host.innerHTML = renderToString(h(App));
  const input = host.querySelector('input'), button = host.querySelector('button');
  const id = input.id; input.value = 'typed early'; input.focus();
  hydrate(h(App), host);
  assert(input === host.querySelector('input') && button === host.querySelector('button'), 'hydration replaced DOM');
  equal(input.value, 'typed early', 'lost input'); equal(input.id, id, 'ID mismatch');
  button.click(); await tick(); equal(button.textContent, 'count:1', 'click did not update');
  update(n => n + 1); update(n => n + 1); await tick(); equal(button.textContent, 'count:3', 'functional updates');
});

await test('keyed fragment moves preserve component state, input identity and clean removed effects', async host => {
  let reorder, cleaned = [];
  function Row({ id }) {
    const [n, set] = useState(0);
    useEffect(() => () => cleaned.push(id), []);
    return h(Fragment, null, h('input', { 'data-id': id, defaultValue: id }), h('button', { onClick: () => set(n + 1) }, id + ':' + n));
  }
  function App() { const [ids, set] = useState(['a', 'b', 'c']); reorder = set; return ids.map(id => h(Row, { key: id, id })); }
  render(h(App), host); await tick();
  const a = host.querySelector('input'); a.value = 'edited';
  host.querySelector('button').click(); await tick();
  reorder(['c', 'a', 'b']); await tick();
  equal([...host.querySelectorAll('button')].map(n => n.textContent), ['c:0', 'a:1', 'b:0'], 'keyed state');
  assert(host.querySelector('[data-id=a]') === a && a.value === 'edited', 'keyed input lost');
  a.focus(); a.setSelectionRange(1, 3);
  reorder(['a', 'b', 'c']); await tick();
  assert(document.activeElement === a && a.selectionStart === 1 && a.selectionEnd === 3, 'keyed move lost focus or selection');
  reorder(['c', 'b']); await tick(); equal(cleaned, ['a'], 'unmount cleanup');
});

await test('effects clean before rerun, layout sees attached refs, setters and memo values stay stable', async host => {
  let set, oldSet, oldRef, oldCallback, calls = [], memoCount = 0;
  function App() {
    const [n, update] = useState(0); set = update;
    const ref = useRef(null), callback = useCallback(() => {}, []);
    useMemo(() => ++memoCount, []);
    if (oldSet) assert(oldSet === set && oldRef === ref && oldCallback === callback, 'unstable hooks');
    oldSet = set; oldRef = ref; oldCallback = callback;
    useLayoutEffect(() => { assert(ref.current.isConnected, 'layout before attach'); calls.push('layout' + n); }, [n]);
    useEffect(() => { calls.push('effect' + n); return () => calls.push('cleanup' + n); }, [n]);
    return h('div', { ref }, n);
  }
  render(h(App), host); await tick(); set(1); await tick(); render(null, host);
  equal(calls, ['layout0', 'effect0', 'layout1', 'cleanup0', 'effect1', 'cleanup1'], 'effect order');
  equal(memoCount, 1, 'memo recomputed'); assert(oldRef.current === null, 'ref not cleared');
  set(2); await tick(); equal(host.textContent, '', 'unmounted setter updated');
});

await test('context crosses memo boundaries and nested providers restore sibling values', async host => {
  const C = createContext('default'); let set; const setters = {};
  const Read = memo(function Read() { const [n, update] = useState(0); const value = useContext(C); setters[value] = update; return h('b', null, value + n); });
  function App() { const [value, update] = useState('outer'); set = update; return h(Fragment, null,
    h(C.Provider, { value }, h(Read), h(C.Provider, { value: 'inner' }, h(Read)), h(Read)), h(Read)); }
  render(h(App), host); equal(host.textContent, 'outer0inner0outer0default0', 'context scope');
  set('changed'); await tick(); equal(host.textContent, 'changed0inner0changed0default0', 'memo context update');
  setters.default(1); await tick(); equal(host.textContent, 'changed0inner0changed0default1', 'memo local update');
});

await test('autoFocus focuses a field that mounts after load, in place or in a portal', async host => {
  let show;
  function App() {
    const [open, set] = useState(0); show = set;
    return h('div', null, h('button', null, 'other'),
      open === 1 ? h('input', { id: 'inline', autoFocus: true }) : null,
      open === 2 ? createPortal(h('input', { id: 'portal', autoFocus: true }), document.body) : null);
  }
  render(h(App), host);
  host.querySelector('button').focus();
  show(1); await tick();
  equal(document.activeElement && document.activeElement.id, 'inline', 'inline autoFocus');
  show(2); await tick();
  equal(document.activeElement && document.activeElement.id, 'portal', 'portal autoFocus');
});

await test('portals inherit context, coexist with host content, update and unmount', async host => {
  const C = createContext('none'), portal = document.createElement('div');
  portal.innerHTML = '<i>external</i>'; document.body.append(portal);
  let set;
  function Child() { return h('b', null, useContext(C)); }
  function App() { const [value, update] = useState('first'); set = update; return h(C.Provider, { value }, createPortal(h(Child), portal)); }
  render(h(App), host); assert(portal.querySelector('i'), 'lost external child'); equal(portal.querySelector('b').textContent, 'first', 'portal context');
  set('second'); await tick(); equal(portal.querySelector('b').textContent, 'second', 'portal update');
  render(null, host); equal(portal.innerHTML, '<i>external</i>', 'portal cleanup'); portal.remove();
});

await test('controlled forms, property removal, SVG, capture events and raw HTML transitions', async host => {
  let set, events = [];
  function App() {
    const [on, update] = useState(true); set = update;
    return h('div', { onClickCapture: () => events.push('capture') },
      h('button', { onClick: () => events.push('bubble'), disabled: false, 'aria-expanded': on }, 'go'),
      h('input', { type: 'checkbox', checked: on }),
      h('select', { value: on ? 'b' : 'a' }, h('option', { value: 'a' }, 'A'), h('option', { value: 'b' }, 'B')),
      h('svg', { viewBox: '0 0 10 10' }, h('path', { strokeWidth: 2 })),
      h('p', on ? { style: { width: 12, opacity: 0.5 }, dangerouslySetInnerHTML: { __html: '<b>raw</b>' } } : {}, on ? null : 'normal'));
  }
  render(h(App), host); host.querySelector('button').click();
  equal(events, ['capture', 'bubble'], 'event order'); equal(host.querySelector('select').value, 'b', 'select initial');
  equal(host.querySelector('p').style.width, '12px', 'style units');
  equal(host.querySelector('path').getAttribute('stroke-width'), '2', 'svg attribute');
  set(false); await tick(); assert(!host.querySelector('input').checked, 'checked update');
  equal(host.querySelector('select').value, 'a', 'select update');
  equal(host.querySelector('button').getAttribute('aria-expanded'), 'false', 'aria false');
  equal(host.querySelector('p').textContent, 'normal', 'raw transition'); equal(host.querySelector('p').style.width, '', 'style removal');
});

await test('forwarded imperative refs, reducer and imperative library children', async host => {
  const ref = { current: null }; let update;
  const Editor = forwardRef(function Editor(props, forwarded) {
    const hostRef = useRef(null), [n, dispatch] = useReducer((n, delta) => n + delta, 1); update = dispatch;
    useImperativeHandle(forwarded, () => ({ count: n }), [n]);
    useEffect(() => { const editor = document.createElement('b'); editor.textContent = 'editor'; hostRef.current.append(editor); return () => editor.remove(); }, []);
    return h('div', { ref: hostRef, 'data-n': n });
  });
  render(h(Editor, { ref }), host); await tick(); const editor = host.querySelector('b');
  equal(ref.current.count, 1, 'imperative initial'); update(2); await tick();
  assert(editor === host.querySelector('b'), 'removed imperative child'); equal(ref.current.count, 3, 'reducer');
  render(null, host); assert(ref.current === null, 'imperative cleanup');
});

await test('hydration keeps the option the server selected from a select defaultValue', async host => {
  function Form() {
    return h('form', null,
      h('select', { name: 'to', defaultValue: 'b' }, h('option', { value: 'a' }, 'A'), h('option', { value: 'b' }, 'B')),
      h('select', { name: 'c', value: 'y', onChange: () => {} }, h('option', { value: 'x' }, 'X'), h('option', { value: 'y' }, 'Y')));
  }
  host.innerHTML = renderToString(h(Form));
  equal(host.querySelector('select[name=to]').value, 'b', 'server markup selects the default');
  hydrate(h(Form), host);
  equal(host.querySelector('select[name=to]').value, 'b', 'hydration dropped the server-selected option');
  equal(host.querySelector('select[name=c]').value, 'y', 'controlled select lost its value on hydration');
});

await test('onChange on text fields fires per input event; checkboxes and selects on change', async host => {
  const seen = [];
  function Form() {
    const [text, setText] = useState('');
    return h('form', null,
      h('input', { name: 'q', value: text, onChange: e => { seen.push('q:' + e.type); setText(e.target.value); } }),
      h('textarea', { name: 't', onChange: e => seen.push('t:' + e.type) }),
      h('input', { name: 'c', type: 'checkbox', onChange: e => seen.push('c:' + e.type) }),
      h('select', { name: 's', onChange: e => seen.push('s:' + e.type) }, h('option', { value: 'a' }, 'A')),
      h('output', null, text));
  }
  render(h(Form), host); await tick();
  const q = host.querySelector('[name=q]');
  for (const ch of 'ab') { q.value += ch; q.dispatchEvent(new Event('input', { bubbles: true })); await tick(); }
  equal(host.querySelector('output').textContent, 'ab', 'controlled input did not update per keystroke');
  q.dispatchEvent(new Event('change', { bubbles: true }));
  host.querySelector('[name=t]').dispatchEvent(new Event('input', { bubbles: true }));
  host.querySelector('[name=c]').click();
  host.querySelector('[name=s]').dispatchEvent(new Event('change', { bubbles: true }));
  equal(seen, ['q:input', 'q:input', 't:input', 'c:change', 's:change'], 'onChange event mapping');
});

await test('hydration recovers missing and extra elements without losing matching siblings', async host => {
  host.innerHTML = '<div><b>keep</b><aside>obsolete</aside></div>';
  const kept = host.querySelector('b');
  hydrate(h('div', null, null, h('i', null, 'new'), h('b', null, 'keep')), host);
  assert(kept === host.querySelector('b'), 'matching sibling replaced');
  equal(host.textContent, 'newkeep', 'mismatch recovery'); assert(!host.querySelector('aside'), 'extra SSR node left');
});

await test('child state does not rerender ancestors or unaffected siblings', async host => {
  let set, parentCalls = 0, siblingCalls = 0;
  function Child() { const [n, update] = useState(0); set = update; return h('b', null, n); }
  function Sibling() { siblingCalls++; return h('i', null, 'sibling'); }
  const Boundary = memo(function Boundary() { return h(Child); }, () => true);
  function App() { parentCalls++; return h('div', null, h(Boundary), h(Sibling)); }
  render(h(App), host); set(1); await tick();
  equal(host.querySelector('b').textContent, '1', 'memo swallowed descendant state');
  equal([parentCalls, siblingCalls], [1, 1], 'unaffected components rerendered');
});

await test('navigation unmount resets the root and hydrates replacement HTML exactly once', async host => {
  let set, cleaned = 0;
  function App() {
    const [n, update] = useState(0); set = update;
    const id = useId();
    useEffect(() => () => cleaned++, []);
    return h('button', { id, onClick: () => update(n + 1) }, n);
  }
  hydrate(h(App), host); await tick(); set(10);
  render(null, host); render(null, host);
  host.innerHTML = renderToString(h(App));
  const button = host.querySelector('button');
  hydrate(h(App), host); await tick();
  assert(host.querySelector('button') === button, 'navigation replaced SSR nodes');
  equal(host.querySelectorAll('button').length, 1, 'navigation duplicated page');
  equal(button.id, 'zeb-0', 'navigation ID mismatch'); equal(button.textContent, '0', 'stale queued update');
  equal(cleaned, 1, 'navigation cleanup'); button.click(); await tick(); equal(button.textContent, '1', 'navigation event handler');
});

const { recoveryTests } = await import('./recovery.mjs');
await recoveryTests({ test, assert, equal, tick });

globalThis.__zebReactTestResults = results;
document.getElementById('results').textContent = JSON.stringify(results, null, 2);
