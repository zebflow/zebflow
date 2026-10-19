// The server stand-in of zeb/world (src/rwe/runtime/zeb_ssr_libs.js), loaded unchanged with the
// real Zeb React behind it. Run from the repository root:
//
//   node --test tests/rwe/runtime/world-stand-in.test.mjs
//
// aslnut has hundreds of names a page calls while it renders; each one is a value that can be
// called, read and awaited on a server. An await that never settles hangs the render, so these
// tests race every await against a timer.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { h, renderToString } from '../../../src/rwe/runtime/zeb_react.mjs';
import '../../../src/rwe/runtime/zeb_ssr_libs.js';

const world = globalThis.__zebLibs['zeb/world'];
const settles = (value) => Promise.race([
  Promise.resolve(value).then(() => 'settled'),
  new Promise((resolve) => setTimeout(() => resolve('hung'), 200)),
]);

test('awaiting a stand-in, or the library itself, settles', async () => {
  assert.equal(await settles(world.earth({ imagery: [world.imagery.sentinel()] })), 'settled');
  assert.equal(await settles(world.data.geojson('/places.geojson')), 'settled');
  assert.equal(await settles(world), 'settled');
});

test('the four keys that must be undefined are', () => {
  const made = world.layer({ id: 'places' });
  for (const value of [made, world, world.scale]) {
    assert.equal(value.then, undefined);
    assert.equal(value.toJSON, undefined);
    assert.equal(value[Symbol.iterator], undefined);
    assert.equal(value[Symbol.toPrimitive], undefined);
  }
  assert.equal(JSON.stringify({ made }), '{}');
});

test('a stand-in answers itself to a call, a construction and a property read', () => {
  const made = world.scale.category((d) => d.props.kind);
  assert.equal(typeof made, 'function');
  assert.equal(made.a.b.c(1, 2).d, made.z);
  assert.equal(typeof new world.theme.dark(), 'function');
});

test('World renders the box the wrapper renders on a server, the hooks answer no world', () => {
  assert.equal(
    renderToString(h(world.World, { className: 'map', style: { height: '300px' } }, h('p', { id: 'overlay' }, 'on top'))),
    '<div class="map" style="position:relative;height:300px"><div style="position:absolute;inset:0"></div><p id="overlay">on top</p></div>');
  assert.equal(renderToString(h(world.World, null)), '<div style="position:relative"><div style="position:absolute;inset:0"></div></div>');
  assert.equal(world.useWorld(), null);
  assert.equal(world.useElement('places'), null);
  assert.equal(world.useWorldEvent('tap', () => {}), undefined);
});

// The stand-in is pinned to the real thing: Zebflow's wrapper inside the built bundle, rendered on a
// server with the same Zeb React, answers the same markup (its effects, which make the world, never run).
test('the stand-in renders exactly what the bundled wrapper renders on a server', async () => {
  const bundled = await import('../../../blessed/rwe-libraries/world/0.1/runtime/world.bundle.mjs');
  const pages = [
    (World) => h(World, { className: 'map', style: { height: '300px' } }, h('p', { id: 'overlay' }, 'on top')),
    (World) => h(World, null),
    (World) => h(World, { headless: true, style: { position: 'absolute', width: '480px' } }),
  ];
  for (const page of pages) assert.equal(renderToString(page(world.World)), renderToString(page(bundled.World)));
});
