import { test } from 'node:test';
import assert from 'node:assert/strict';
import { h, Fragment, ErrorBoundary, useSyncExternalStore, renderToString, createContext, useContext, useId, useState, useEffect } from '../../../src/rwe/runtime/zeb_react.mjs';

test('SSR escapes content and attributes, preserves aria false and CSS units', () => {
  assert.equal(renderToString(h('div', { title: '<"&', 'aria-hidden': false, style: { width: 12, opacity: 0.5, '--n': 2 } }, '<&')),
    '<div title="&lt;&quot;&amp;" aria-hidden="false" style="width:12px;opacity:0.5;--n:2">&lt;&amp;</div>');
  assert.equal(renderToString(h('input', { disabled: false, checked: true, onClick() {} })), '<input checked="">');
});

test('nested providers restore sibling context and do not leak between requests', () => {
  const C = createContext('default');
  function Read() { return h('b', null, useContext(C)); }
  const tree = h(Fragment, null, h(C.Provider, { value: 'outer' },
    h(Read), h(C.Provider, { value: 'inner' }, h(Read)), h(Read)), h(Read));
  assert.equal(renderToString(tree), '<b>outer</b><b>inner</b><b>outer</b><b>default</b>');
  assert.equal(renderToString(h(Read)), '<b>default</b>');
});

test('SSR hook initializers, unique deterministic IDs, and effect suppression', () => {
  function Field() {
    const id = useId(), [value] = useState(() => 'initial');
    useEffect(() => { throw new Error('SSR ran an effect'); });
    return h('input', { id, value });
  }
  const tree = h(Fragment, null, h(Field), h(Field));
  const html = '<input id="zeb-0" value="initial"><input id="zeb-1" value="initial">';
  assert.equal(renderToString(tree), html);
  assert.equal(renderToString(tree), html);
});

test('SSR forms, SVG, fragments and raw HTML', () => {
  assert.equal(renderToString(h('select', { value: 'b' }, h('option', { value: 'a' }, 'A'), h('option', { value: 'b' }, 'B'))),
    '<select><option value="a">A</option><option value="b" selected="">B</option></select>');
  assert.equal(renderToString(h('textarea', { defaultValue: '<hello>' })), '<textarea>&lt;hello&gt;</textarea>');
  assert.equal(renderToString(h('svg', { viewBox: '0 0 10 10' }, h('path', { strokeWidth: 2 }))),
    '<svg viewBox="0 0 10 10"><path stroke-width="2"></path></svg>');
  assert.equal(renderToString(h('div', { dangerouslySetInnerHTML: { __html: '<b>trusted</b>' } })), '<div><b>trusted</b></div>');
});

test('invalid components and attributes fail explicitly', () => {
  assert.throws(() => renderToString(h('div><script')), /invalid tag/);
  assert.throws(() => renderToString(h('div', { 'x onclick': 'bad' })), /invalid attribute/);
  assert.throws(() => useState(0), /inside a component/);
});

test('boundaries render escaped fallbacks with scoped context and deterministic IDs', () => {
  const C = createContext('outside'); const caught = [];
  function Broken() { useId(); throw new Error('<broken>'); }
  function Fallback({ error }) { return h('b', { id: useId() }, useContext(C), error.message); }
  function After() { return h('i', { id: useId() }, 'after'); }
  const tree = h(Fragment, null, h(C.Provider, { value: 'inside' }, h(ErrorBoundary, {
    fallbackRender: ({ error }) => h(Fallback, { error }),
    onError: (error, info) => caught.push([error.message, info.componentStack]),
  }, h(Broken))), h(After));
  for (let n = 0; n < 2; n++) assert.equal(renderToString(tree, { onError: () => 'legacy-error' }),
    '<b id="zeb-0">inside&lt;broken&gt;</b><i id="zeb-1">after</i>');
  assert.equal(caught.length, 2);
  assert.match(caught[0][1], /at Broken\n    at ErrorBoundary/);
});

test('failed fallbacks and reporting callbacks escalate; arbitrary thrown values survive', () => {
  function Broken() { throw null; }
  const outer = child => h(ErrorBoundary, { fallbackRender: ({ error }) => String(error) }, child);
  assert.equal(renderToString(outer(h(Broken))), 'null');
  assert.equal(renderToString(outer(h(ErrorBoundary, {
    fallbackRender() { throw new Error('fallback failed'); },
  }, h(Broken)))), 'Error: fallback failed');
  assert.equal(renderToString(outer(h(ErrorBoundary, {
    fallback: 'unused', onError() { throw new Error('report failed'); },
  }, h(Broken)))), 'Error: report failed');
  assert.equal(renderToString(h(ErrorBoundary, { fallback: null }, h(Broken))), '');
  assert.throws(() => renderToString(h(ErrorBoundary, {}, h(() => { throw new Error('unhandled'); }))), /unhandled/);
});

test('external stores read only the server snapshot during SSR without subscribing', () => {
  let serverReads = 0;
  function Read() {
    return h('b', null, useSyncExternalStore(
      () => { throw new Error('subscribed on server'); },
      () => { throw new Error('browser getter on server'); },
      () => { serverReads++; return 'server'; }));
  }
  assert.equal(renderToString(h(Read)), '<b>server</b>');
  assert.ok(serverReads > 0);
  assert.equal(renderToString(h(Read)), '<b>server</b>');
});

test('missing server snapshots and uncached snapshots are explicit recoverable errors', () => {
  function Missing() { return useSyncExternalStore(() => () => {}, () => 1); }
  function Uncached() { return useSyncExternalStore(() => () => {}, () => 1, () => ({})); }
  assert.throws(() => renderToString(h(Missing)), /requires getServerSnapshot/);
  assert.throws(() => renderToString(h(Uncached)), /cached snapshot/);
  assert.equal(renderToString(h(ErrorBoundary, { fallback: h('b', null, 'offline') }, h(Missing))), '<b>offline</b>');
});
