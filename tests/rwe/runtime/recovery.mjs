import { h, Fragment, ErrorBoundary, useSyncExternalStore, render, hydrate, renderToString,
  useState, useEffect, useLayoutEffect, useId, memo, createPortal } from '../../../src/rwe/runtime/zeb_react.mjs';

function store(initial) {
  let value = initial; const listeners = new Set();
  return {
    listeners, getSnapshot: () => value,
    subscribe(fn) { listeners.add(fn); return () => listeners.delete(fn); },
    set(next) { value = next; for (const fn of [...listeners]) fn(); },
  };
}

export async function recoveryTests({ test, assert, equal, tick }) {
  await test('boundaries isolate update failures, release resources, and retry with fresh state', async host => {
    let fail, shouldFail = false, cleaned = 0, reports = [], resets = [];
    const ref = { current: null }, external = store(0);
    function Panel() {
      const [n, set] = useState(0); fail = () => { shouldFail = true; set(1); };
      useSyncExternalStore(external.subscribe, external.getSnapshot);
      useEffect(() => () => cleaned++, []);
      if (shouldFail) throw new Error('panel failed');
      return h('b', { ref }, 'panel:' + n);
    }
    function Sibling() { const [n, set] = useState(0); return h('button', { onClick: () => set(n + 1) }, 'sibling:' + n); }
    const tree = h(Fragment, null, h(ErrorBoundary, {
      fallbackRender: ({ error, resetErrorBoundary }) => h('button', { 'data-retry': '', onClick: () => resetErrorBoundary('retry') }, error.message),
      onError: (error, info) => reports.push(info.componentStack),
      onReset: info => { resets.push(info); shouldFail = false; },
    }, h(Panel)), h(Sibling));
    render(tree, host); await tick();
    const sibling = host.querySelector('button'); sibling.click(); await tick();
    fail(); await tick();
    equal(host.textContent, 'panel failedsibling:1', 'isolated fallback');
    assert(sibling === host.querySelectorAll('button')[1], 'sibling DOM replaced');
    assert(ref.current === null && external.listeners.size === 0 && cleaned === 1, 'failed subtree leaked');
    assert(reports.length === 1 && reports[0].includes('at Panel'), 'missing component diagnostics');
    host.querySelector('[data-retry]').click(); await tick();
    equal(host.textContent, 'panel:0sibling:1', 'retry did not remount fresh');
    equal(resets, [{ reason: 'imperative-api', args: ['retry'] }], 'reset callback');
    equal(external.listeners.size, 1, 'retry subscription');
  });

  await test('resetKeys compare values and a broken retry stays contained', async host => {
    let update, broken = true, reports = 0, resets = 0;
    function Broken() { if (broken) throw new Error('broken'); return 'healthy'; }
    function App() {
      const [key, set] = useState(0); update = set;
      return h(ErrorBoundary, {
        resetKeys: [key], onReset: () => resets++, onError: () => reports++,
        fallbackRender: ({ resetErrorBoundary }) => h('button', { onClick: resetErrorBoundary }, 'retry'),
      }, h(Broken));
    }
    render(h(App), host); render(h(App), host);
    equal([reports, resets], [1, 0], 'same reset values retried');
    host.querySelector('button').click(); await tick();
    equal([reports, resets], [2, 1], 'broken retry escaped or looped');
    broken = false; update(1); await tick();
    equal(host.textContent, 'healthy', 'key reset failed'); equal(resets, 2, 'key reset callback');
  });

  await test('partial initial renders discard portal work and never run abandoned effects or refs', async host => {
    const portal = document.createElement('div'); portal.innerHTML = '<i>external</i>'; document.body.append(portal);
    let effects = 0, attached = 0;
    function Before() {
      useEffect(() => effects++, []); useLayoutEffect(() => effects++, []);
      return h(Fragment, null, h('b', { ref: value => { if (value) attached++; } }, 'partial'),
        createPortal(h('b', null, 'portal'), portal));
    }
    function Broken() { throw new Error('initial'); }
    render(h(ErrorBoundary, { fallback: 'fallback' }, h(Before), h(Broken)), host); await tick();
    equal([host.textContent, portal.innerHTML, effects, attached], ['fallback', '<i>external</i>', 0, 0], 'abandoned work survived');
    portal.remove();
  });

  await test('nested boundaries escalate fallback, onError and onReset failures', async host => {
    function Broken() { throw new Error('child'); }
    for (const mode of ['fallback', 'report', 'reset']) {
      const props = {
        fallbackRender: ({ resetErrorBoundary }) => {
          if (mode === 'fallback') throw new Error(mode);
          return h('button', { onClick: resetErrorBoundary }, 'retry');
        },
        onError: () => { if (mode === 'report') throw new Error(mode); },
        onReset: () => { throw new Error(mode); },
      };
      render(h(ErrorBoundary, { fallbackRender: ({ error }) => 'outer:' + error.message },
        h(ErrorBoundary, props, h(Broken))), host);
      if (mode === 'reset') { host.querySelector('button').click(); await tick(); }
      equal(host.textContent, 'outer:' + mode, 'nested ' + mode);
      render(null, host);
    }
  });

  await test('layout, passive, ref and cleanup failures recover at the nearest mounted boundary', async host => {
    for (const mode of ['layout', 'effect', 'ref', 'cleanup', 'unsubscribe']) {
      let hide;
      function Child() {
        if (mode === 'layout') useLayoutEffect(() => { throw new Error(mode); }, []);
        if (mode === 'effect') useEffect(() => { throw new Error(mode); }, []);
        if (mode === 'cleanup') useEffect(() => () => { throw new Error(mode); }, []);
        if (mode === 'unsubscribe') useSyncExternalStore(() => () => { throw new Error(mode); }, () => 0);
        return h('b', { ref: mode === 'ref' ? value => { if (value) throw new Error(mode); } : null }, 'child');
      }
      function Parent() { const [show, set] = useState(true); hide = () => set(false); return show ? h(Child) : 'removed'; }
      render(h(ErrorBoundary, { fallbackRender: ({ error }) => 'caught:' + error.message }, h(Parent)), host);
      await tick();
      if (mode === 'cleanup' || mode === 'unsubscribe') { hide(); await tick(); }
      equal(host.textContent, 'caught:' + mode, 'commit recovery ' + mode);
      render(null, host);
    }
  });

  await test('SSR fallback hydrates, preserves siblings and IDs, then retries in the browser', async host => {
    let broken = true;
    function Broken() { useId(); if (broken) throw new Error('failed'); return h('b', null, 'recovered'); }
    function Fallback({ resetErrorBoundary }) { return h('button', { id: useId(), onClick: resetErrorBoundary }, 'retry'); }
    function Sibling() { return h('input', { id: useId(), defaultValue: 'keep' }); }
    const tree = h(Fragment, null, h(ErrorBoundary, {
      fallbackRender: args => h(Fallback, args), onReset: () => { broken = false; },
    }, h(Broken)), h(Sibling));
    host.innerHTML = renderToString(tree); const button = host.querySelector('button'), input = host.querySelector('input');
    hydrate(tree, host);
    assert(button === host.querySelector('button') && input === host.querySelector('input'), 'fallback hydration replaced nodes');
    equal([button.id, input.id], ['zeb-0', 'zeb-1'], 'fallback hydration IDs');
    button.click(); await tick(); equal(host.querySelector('b').textContent, 'recovered', 'hydrated retry');
    assert(input === host.querySelector('input'), 'retry replaced sibling');
  });

  await test('external store notifications synchronously update memoized subscribers without tearing', async host => {
    const external = store(0); let renders = 0;
    const Read = memo(function Read() { renders++; return h('b', null, useSyncExternalStore(external.subscribe, external.getSnapshot)); });
    render(h(Fragment, null, h(Read), h(Read)), host);
    equal(external.listeners.size, 2, 'missing subscribers');
    external.set(1); equal(host.textContent, '11', 'notification was deferred or tore');
    const count = renders; external.set(1); equal(renders, count, 'equal snapshot rerendered');
    render(null, host); equal(external.listeners.size, 0, 'unmount subscriptions leaked');
    external.set(2); await tick(); equal(host.textContent, '', 'unmounted listener updated');
  });

  await test('external stores resubscribe only when subscribe changes and ignore stale listeners', async host => {
    const a = store('a'), b = store('b'); let update, renders = 0;
    function App() {
      renders++; const [which, set] = useState(false); update = set;
      const source = which ? b : a;
      return h('b', null, useSyncExternalStore(source.subscribe, () => source.getSnapshot()));
    }
    render(h(App), host); const stale = [...a.listeners][0];
    render(h(App), host); assert(a.listeners.has(stale), 'getter change resubscribed');
    update(true); await tick(); equal([a.listeners.size, b.listeners.size, host.textContent], [0, 1, 'b'], 'store replacement');
    const count = renders; stale(); equal(renders, count, 'stale callback was live');
    b.set('new'); equal(host.textContent, 'new', 'new subscription inactive');
  });

  await test('stores close render-to-subscribe races and changes from layout effects', async host => {
    const external = store(0); let subscriptions = 0;
    const subscribe = fn => { subscriptions++; external.set(1); return external.subscribe(fn); };
    function Read() {
      const value = useSyncExternalStore(subscribe, external.getSnapshot);
      useLayoutEffect(() => { external.set(2); }, []);
      return h('b', null, value);
    }
    render(h(Read), host); equal(host.textContent, '2', 'missed subscription or layout change');
    equal(subscriptions, 1, 'resubscribed during catch-up');
  });

  await test('hydration uses the server snapshot before synchronously catching up to the live store', async host => {
    const external = store('live'); const commits = [];
    function Read() {
      const value = useSyncExternalStore(external.subscribe, external.getSnapshot, () => 'server');
      useLayoutEffect(() => { commits.push(value); }, [value]);
      return h('b', null, value);
    }
    host.innerHTML = renderToString(h(Read)); const original = host.querySelector('b');
    equal(host.textContent, 'server', 'SSR used live snapshot');
    hydrate(h(Read), host);
    equal(commits, ['server', 'live'], 'hydration snapshot handoff');
    assert(original === host.querySelector('b'), 'store hydration replaced DOM');
    equal(host.textContent, 'live', 'hydration did not catch up');
  });

  await test('snapshot and subscribe errors recover, unsubscribe, and retry', async host => {
    const external = store(0); let broken = false, subscriptionBroken = false;
    function Read() { return useSyncExternalStore(
      subscriptionBroken ? () => { throw new Error('subscribe'); } : external.subscribe,
      () => { if (broken) throw new Error('snapshot'); return external.getSnapshot(); }); }
    const tree = h(ErrorBoundary, {
      fallbackRender: ({ error, resetErrorBoundary }) => h('button', { onClick: resetErrorBoundary }, error.message),
      onReset: () => { broken = false; subscriptionBroken = false; },
    }, h(Read));
    render(tree, host); broken = true; external.set(1);
    equal(host.textContent, 'snapshot', 'snapshot failure escaped'); equal(external.listeners.size, 0, 'failed snapshot leaked');
    host.querySelector('button').click(); await tick(); equal(host.textContent, '1', 'snapshot recovery');
    render(null, host); subscriptionBroken = true; render(tree, host);
    equal(host.textContent, 'subscribe', 'subscribe failure escaped');
    host.querySelector('button').click(); await tick(); equal(host.textContent, '1', 'subscribe recovery');
  });

  await test('invalid uncached snapshots fail once inside a boundary rather than looping', async host => {
    let calls = 0;
    function Bad() { return useSyncExternalStore(() => () => {}, () => { calls++; return {}; }); }
    render(h(ErrorBoundary, { fallbackRender: ({ error }) => error.message }, h(Bad)), host); await tick();
    assert(host.textContent.includes('cached snapshot') && calls < 10, 'unstable snapshot loop');
  });
}
