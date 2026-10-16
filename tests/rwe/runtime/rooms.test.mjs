// zeb/rooms: the shipped bundle, loaded unchanged, against a fake WebSocket,
// a fake fetch, a hand-driven clock and a small hook host standing in for
// Zeb React. Run from the repository root:
//
//   node --test tests/rwe/runtime/rooms.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const BUNDLE = new URL('../../../blessed/rwe-libraries/rooms/0.1/runtime/rooms.bundle.mjs', import.meta.url);

// ── A hook host: just enough of Zeb React to mount, update and unmount ─────
// The bundle takes its hooks from globalThis when it loads, as on a page.

let host = null;
globalThis.useState = (init) => host.useState(init);
globalThis.useEffect = (effect, deps) => host.useEffect(effect, deps);
globalThis.useRef = (init) => host.useRef(init);
globalThis.useCallback = (fn, deps) => host.useCallback(fn, deps);

// Importing must touch no browser global: none exists yet.
assert.equal(globalThis.WebSocket, undefined);
const { joinRoom, useRoom } = await import(BUNDLE);

const sameDeps = (a, b) => a && b && a.length === b.length && a.every((value, i) => Object.is(value, b[i]));

function mount(component, initialProps) {
  const slots = [];
  let index = 0;
  let pending = [];
  let busy = false;
  let dirty = false;
  let gone = false;
  let props = initialProps;
  const mounted = { renders: 0, output: undefined };
  const hooks = {
    useState(init) {
      const slot = (slots[index] ??= { value: typeof init === 'function' ? init() : init });
      index += 1;
      return [slot.value, (update) => {
        const next = typeof update === 'function' ? update(slot.value) : update;
        if (Object.is(next, slot.value)) return;
        slot.value = next;
        run();
      }];
    },
    useRef(init) {
      const slot = (slots[index] ??= { current: init });
      index += 1;
      return slot;
    },
    useCallback(fn, deps) {
      const slot = slots[index];
      if (!slot || !sameDeps(slot.deps, deps)) slots[index] = { fn, deps };
      index += 1;
      return slots[index - 1].fn;
    },
    useEffect(effect, deps) {
      const slot = slots[index];
      if (!slot || !sameDeps(slot.deps, deps)) pending.push({ at: index, effect, deps });
      index += 1;
    },
  };
  function run() {
    if (gone) return;
    if (busy) {
      dirty = true;
      return;
    }
    busy = true;
    do {
      dirty = false;
      index = 0;
      pending = [];
      host = hooks;
      mounted.output = component(props);
      host = null;
      mounted.renders += 1;
      for (const { at, effect, deps } of pending) {
        slots[at]?.cleanup?.();
        slots[at] = { deps, cleanup: null };
        const cleanup = effect();
        slots[at].cleanup = typeof cleanup === 'function' ? cleanup : null;
      }
    } while (dirty && !gone);
    busy = false;
  }
  mounted.update = (next) => {
    props = next;
    run();
  };
  mounted.unmount = () => {
    gone = true;
    for (const slot of slots) slot?.cleanup?.();
  };
  run();
  return mounted;
}

// ── A fake network ─────────────────────────────────────────────────────────

class FakeSocket {
  static opened = [];
  constructor(url, protocol) {
    this.url = url;
    this.protocol = protocol;
    this.readyState = 1;
    this.sent = [];
    this.closedWith = null;
    FakeSocket.opened.push(this);
  }
  send(text) {
    this.sent.push(JSON.parse(text));
  }
  close(code) {
    this.closedWith = code;
    this.readyState = 3;
  }
  // The server's side.
  frame(frame) {
    this.onmessage?.({ data: typeof frame === 'string' ? frame : JSON.stringify(frame) });
  }
  drop(code = 1006, reason = '') {
    this.readyState = 3;
    this.onclose?.({ code, reason });
  }
}

// Fresh globals for one test: a page at play.example.com whose server wrote
// `base` as its room base (null: the ws surface is off), and a clock that
// only moves when the test says so.
function page({ base = '/_ws/rooms/', ticketStatus = 200 } = {}) {
  FakeSocket.opened = [];
  globalThis.WebSocket = FakeSocket;
  globalThis.location = { protocol: 'https:', host: 'play.example.com' };
  globalThis.document = {
    getElementById: (id) => (id === '__rwe_payload' && base !== null ? { getAttribute: (name) => (name === 'data-rooms' ? base : null) } : null),
  };
  const posts = [];
  globalThis.fetch = async (url, init) => {
    posts.push({ url, init });
    return { ok: ticketStatus === 200, status: ticketStatus, json: async () => ({ ticket: `t-${posts.length}` }) };
  };
  const timers = new Map();
  let next = 0;
  const real = { setTimeout: globalThis.setTimeout, clearTimeout: globalThis.clearTimeout };
  globalThis.setTimeout = (fn, ms) => {
    timers.set(++next, { fn, ms });
    return next;
  };
  globalThis.clearTimeout = (id) => timers.delete(id);
  return {
    posts,
    timers,
    elapse() {
      const due = [...timers.values()];
      timers.clear();
      for (const timer of due) timer.fn();
    },
    restore() {
      Object.assign(globalThis, real);
      delete globalThis.WebSocket;
      delete globalThis.document;
      delete globalThis.location;
      delete globalThis.fetch;
    },
  };
}

// Let the library's promises and microtasks run.
const settle = () => new Promise((resolve) => setImmediate(resolve));
const last = () => FakeSocket.opened[FakeSocket.opened.length - 1];

async function joined(room, state = {}, extra = {}) {
  await settle();
  last().frame({ type: 'joined', session_id: 'ws-1', room: room.name, state, ...extra });
  assert.equal(room.status, 'joined');
}

// ── Patches ────────────────────────────────────────────────────────────────

test('patches set and delete at object and array paths, sharing everything untouched', async () => {
  const env = page();
  try {
    const room = joinRoom('match-7');
    await joined(room, { players: { a: { x: 0 }, b: { x: 5 } }, log: [{ n: 1 }, { n: 2 }], round: 1 });
    const start = room.state;

    last().frame({ type: 'patch', ops: [
      { op: 'set', path: ['players', 'a', 'x'], value: 3 },
      { op: 'set', path: ['players', 'c'], value: { x: 9 } },
      { op: 'set', path: ['log', 1, 'n'], value: 20 },
      { op: 'set', path: ['zone', 'forest', 'level'], value: 3 },
    ] });
    const after = room.state;
    assert.notEqual(after, start, 'the root is new');
    assert.deepEqual(after.players.a, { x: 3 });
    assert.deepEqual(after.players.c, { x: 9 });
    assert.deepEqual(after.zone, { forest: { level: 3 } }, 'missing objects are created on the way');
    assert.deepEqual(after.log, [{ n: 1 }, { n: 20 }]);
    assert.equal(after.players.b, start.players.b, 'an untouched sibling keeps its identity');
    assert.equal(after.log[0], start.log[0], 'an untouched array item keeps its identity');
    assert.deepEqual(start.players.a, { x: 0 }, 'the old state is never changed');

    last().frame({ type: 'patch', ops: [
      { op: 'del', path: ['players', 'b'] },
      { op: 'del', path: ['log', 0] },
      { op: 'del', path: ['nothing', 'here'] },
    ] });
    assert.deepEqual(Object.keys(room.state.players), ['a', 'c']);
    assert.deepEqual(room.state.log, [{ n: 20 }]);
    assert.equal(room.state.players.a, after.players.a);
    assert.equal(room.state.zone, after.zone);

    const before = room.state;
    last().frame({ type: 'patch', ops: [{ op: 'set', path: ['round'], value: 1 }] });
    assert.equal(room.state, before, 'setting the same value changes nothing');

    last().frame({ type: 'resync', state: { players: {} } });
    assert.deepEqual(room.state, { players: {} }, 'resync replaces the state');
    room.leave();
  } finally {
    env.restore();
  }
});

test('a path through __proto__, constructor or prototype is ignored with its op', async () => {
  const env = page();
  try {
    const room = joinRoom('match-7');
    await joined(room, { a: {} });
    last().frame({ type: 'patch', ops: [
      { op: 'set', path: ['__proto__', 'polluted'], value: true },
      { op: 'set', path: ['a', 'constructor', 'prototype', 'polluted'], value: true },
      { op: 'set', path: ['a', 'prototype'], value: true },
      { op: 'del', path: ['__proto__', 'toString'] },
      { op: 'set', path: ['a', 'ok'], value: 1 },
    ] });
    assert.equal(({}).polluted, undefined);
    assert.equal(Object.prototype.toString.name, 'toString');
    assert.deepEqual(room.state, { a: { ok: 1 } }, 'the safe op beside them still applies');
    room.leave();
  } finally {
    env.restore();
  }
});

// ── The connection ─────────────────────────────────────────────────────────

test('a dropped socket goes away, resumes with its token and is joined again', async () => {
  const env = page();
  try {
    const seen = [];
    const tickets = [];
    const room = joinRoom('match-7', { ticket: () => { tickets.push(1); return 'tk-a'; } });
    room.onStatus((status) => seen.push(status));
    await settle();
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/match-7?ticket=tk-a');
    assert.equal(last().protocol, 'zebflow.rooms.v1');
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: { n: 1 }, resume: 'r-1' });

    last().drop(1006);
    assert.equal(room.status, 'away');
    assert.equal(env.timers.size, 1);
    const [{ ms }] = env.timers.values();
    assert.ok(ms >= 125 && ms <= 250, `the first wait is about 250 ms: ${ms}`);
    env.elapse();
    await settle();
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/match-7?resume=r-1');
    assert.equal(tickets.length, 1, 'a resume never fetches a ticket');
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: { n: 2 }, resume: 'r-2' });
    assert.equal(room.status, 'joined');
    assert.equal(room.sessionId, 'ws-1');
    assert.deepEqual(room.state, { n: 2 });
    assert.deepEqual(seen, ['joined', 'away', 'joined']);
    room.leave();
  } finally {
    env.restore();
  }
});

test('a refused resume makes one fresh join with a new ticket', async () => {
  const env = page();
  try {
    let issued = 0;
    const room = joinRoom('match-7', { ticket: () => `tk-${++issued}` });
    await joined(room, {}, { resume: 'r-1' });
    last().drop(1006);
    env.elapse();
    await settle();
    assert.match(last().url, /\?resume=r-1$/);
    last().drop(4401, 'unauthorized');
    await settle();
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/match-7?ticket=tk-2');
    assert.equal(issued, 2);
    assert.equal(room.status, 'away');
    last().frame({ type: 'joined', session_id: 'ws-9', room: 'match-7', state: {} });
    assert.equal(room.sessionId, 'ws-9');

    // That fresh join refused too: closed, not another round.
    last().drop(1006);
    env.elapse();
    await settle();
    last().drop(4401, 'unauthorized');
    await settle();
    assert.equal(room.status, 'closed');
    assert.deepEqual(room.closed, { code: 4401, reason: 'unauthorized' });
  } finally {
    env.restore();
  }
});

test('every refusal closes the room with its code and is never retried', async () => {
  for (const code of [4401, 4404, 4408, 4409, 4410, 4429, 1008]) {
    const env = page();
    try {
      const details = [];
      const room = joinRoom('match-7');
      room.onStatus((status, detail) => details.push([status, detail]));
      await joined(room, {}, { resume: 'r-1' });
      const sockets = FakeSocket.opened.length;
      last().drop(code, 'why');
      await settle();
      assert.equal(room.status, 'closed', `${code}`);
      assert.deepEqual(details.at(-1), ['closed', { code, reason: 'why' }]);
      assert.equal(env.timers.size, 0, `${code} schedules no retry`);
      assert.equal(FakeSocket.opened.length, sockets, `${code} opens no socket`);
    } finally {
      env.restore();
    }
  }
});

test('leave closes normally and never reconnects', async () => {
  const env = page();
  try {
    const room = joinRoom('match-7');
    await joined(room, {}, { resume: 'r-1' });
    const socket = last();
    room.leave();
    assert.equal(socket.closedWith, 1000);
    assert.deepEqual(room.closed, { code: 1000, reason: 'left' });
    socket.drop(1000);
    await settle();
    assert.equal(env.timers.size, 0);
    assert.equal(FakeSocket.opened.length, 1);
    assert.equal(room.status, 'closed');
  } finally {
    env.restore();
  }
});

test('send is false unless joined, refuses $ names and queues nothing', async () => {
  const env = page();
  try {
    const room = joinRoom('match-7');
    assert.equal(room.send('move', { x: 1 }), false, 'connecting');
    await settle();
    assert.equal(room.send('move', { x: 1 }), false, 'socket open, not joined yet');
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: {}, resume: 'r-1' });
    assert.equal(room.send('move', { x: 3, y: 7 }), true);
    assert.equal(room.send('$connect', {}), false);
    assert.equal(room.send('', {}), false);
    assert.deepEqual(last().sent, [{ event: 'move', payload: { x: 3, y: 7 } }]);
    last().drop(1006);
    assert.equal(room.send('move', {}), false, 'away');
    env.elapse();
    await settle();
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: {} });
    assert.deepEqual(last().sent, [], 'nothing sent while away arrives later');
    room.leave();
    assert.equal(room.send('move', {}), false, 'closed');
  } finally {
    env.restore();
  }
});

test('a tick frame applies its ops before it delivers its events, in order', async () => {
  const env = page();
  try {
    const seen = [];
    const room = joinRoom('match-7', { onEvent: (event) => seen.push(`any:${event}`) });
    room.on('hit', (payload) => seen.push(`hit:${payload.damage}:hp=${room.state.hp}`));
    const off = room.on('miss', () => seen.push('miss'));
    off();
    await joined(room, { hp: 10 });
    last().frame({ type: 'patch', tick: 412, ops: [{ op: 'set', path: ['hp'], value: 7 }],
      events: [{ event: 'hit', payload: { damage: 3 } }, { event: 'miss', payload: {} }] });
    last().frame({ type: 'event', event: 'hit', payload: { damage: 1 } });
    last().frame({ type: 'unknown-kind', anything: true });
    last().frame('not json');
    assert.deepEqual(seen, ['hit:3:hp=7', 'any:hit', 'any:miss', 'hit:1:hp=7', 'any:hit']);
    room.leave();
  } finally {
    env.restore();
  }
});

// ── Tickets and where rooms live ───────────────────────────────────────────

test('a ticket URL is POSTed for every fresh join; an open room sends none', async () => {
  const env = page();
  try {
    const room = joinRoom('match-7', { ticket: '/api/matches/7/ticket' });
    await settle();
    assert.equal(env.posts.length, 1);
    assert.equal(env.posts[0].url, '/api/matches/7/ticket');
    assert.equal(env.posts[0].init.method, 'POST');
    assert.equal(env.posts[0].init.credentials, 'same-origin');
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/match-7?ticket=t-1');
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: {} });
    last().drop(1006); // no resume token: the seat is gone, a fresh join follows
    env.elapse();
    await settle();
    assert.equal(env.posts.length, 2);
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/match-7?ticket=t-2');
    room.leave();

    const open = joinRoom('lobby');
    await settle();
    assert.equal(last().url, 'wss://play.example.com/_ws/rooms/lobby');
    assert.equal(env.posts.length, 2);
    open.leave();
  } finally {
    env.restore();
  }
});

test('a failed ticket fetch closes the room with the HTTP status', async () => {
  const env = page({ ticketStatus: 403 });
  try {
    const room = joinRoom('match-7', { ticket: '/api/matches/7/ticket' });
    await settle();
    assert.deepEqual(room.closed, { code: null, reason: 'ticket', status: 403 });
    assert.equal(FakeSocket.opened.length, 0);
    const thrown = joinRoom('match-8', { ticket: async () => { throw new Error('no'); } });
    await settle();
    assert.deepEqual(thrown.closed, { code: null, reason: 'ticket', status: 0 });
    const raw = joinRoom('match-9', { ticket: { value: 'tk' } });
    await settle();
    assert.equal(raw.closed.reason, 'ticket', 'a raw ticket value is not a ticket option');
  } finally {
    env.restore();
  }
});

test('with the ws surface off the room closes as rooms-off and opens no socket', async () => {
  const env = page({ base: null });
  try {
    const room = joinRoom('match-7');
    await settle();
    assert.equal(room.status, 'closed');
    assert.deepEqual(room.closed, { code: null, reason: 'rooms-off' });
    assert.equal(FakeSocket.opened.length, 0);
  } finally {
    env.restore();
  }
});

test('the base is made absolute; url overrides it; several rooms each get a socket', async () => {
  const env = page({ base: '/ws/site-a/demo/rooms/' });
  try {
    globalThis.location = { protocol: 'http:', host: 'localhost:8080' };
    const a = joinRoom('match-7');
    const b = joinRoom('match-7');
    const outside = joinRoom('zone forest/3', { url: 'https://play.example.com/_ws/rooms' });
    await settle();
    assert.deepEqual(FakeSocket.opened.map((socket) => socket.url), [
      'ws://localhost:8080/ws/site-a/demo/rooms/match-7',
      'ws://localhost:8080/ws/site-a/demo/rooms/match-7',
      'wss://play.example.com/_ws/rooms/zone%20forest%2F3',
    ]);
    for (const room of [a, b, outside]) room.leave();
  } finally {
    env.restore();
  }
});

// ── useRoom ────────────────────────────────────────────────────────────────

test('useRoom joins in an effect and re-renders only when the selected part changes', async () => {
  const env = page();
  try {
    const view = mount((props) => useRoom(props.name, { select: (state) => state.players }), { name: 'match-7' });
    assert.equal(view.output.status, 'connecting');
    assert.equal(view.output.state, undefined);
    assert.equal(view.output.send('move', {}), false);
    await settle();
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: { players: { a: 1 }, clock: 0 } });
    assert.equal(view.output.status, 'joined');
    assert.deepEqual(view.output.state, { a: 1 });
    const renders = view.renders;

    last().frame({ type: 'patch', ops: [{ op: 'set', path: ['clock'], value: 1 }] });
    assert.equal(view.renders, renders, 'a change outside the selection renders nothing');
    last().frame({ type: 'patch', ops: [{ op: 'set', path: ['players', 'a'], value: 2 }] });
    assert.equal(view.renders, renders + 1);
    assert.deepEqual(view.output.state, { a: 2 });

    assert.equal(view.output.send('move', { x: 1 }), true);
    assert.deepEqual(last().sent, [{ event: 'move', payload: { x: 1 } }]);

    const first = last();
    view.update({ name: 'match-8' });
    assert.equal(first.closedWith, 1000, 'a new room name leaves the old room');
    await settle();
    assert.match(last().url, /\/match-8$/);
    view.unmount();
    assert.equal(last().closedWith, 1000, 'unmounting leaves the room');
  } finally {
    env.restore();
  }
});

test('useRoom calls the latest onEvent and fetches the latest ticket', async () => {
  const env = page();
  try {
    const seen = [];
    const view = mount((props) => useRoom('match-7', props), { onEvent: (event) => seen.push(`old:${event}`), ticket: () => 'tk-old' });
    view.update({ onEvent: (event, payload) => seen.push(`new:${event}:${payload.text}`), ticket: () => 'tk-new' });
    await settle();
    assert.match(last().url, /\?ticket=tk-new$/);
    last().frame({ type: 'joined', session_id: 'ws-1', room: 'match-7', state: {} });
    last().frame({ type: 'event', event: 'chat.message', payload: { text: 'hi' } });
    assert.deepEqual(seen, ['new:chat.message:hi']);
    view.unmount();
  } finally {
    env.restore();
  }
});

test('the bundle carries no eval and no new Function', () => {
  const source = readFileSync(BUNDLE, 'utf8');
  assert.doesNotMatch(source, /\beval\s*\(/);
  assert.doesNotMatch(source, /new\s+Function\b/);
  assert.doesNotMatch(source, /\bimport\s*\(/, 'no dynamic import');
});
