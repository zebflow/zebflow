// zeb/rooms 0.1 — join a Zebflow room from a page, or from any script.
//
// Speaks the wire protocol zebflow.rooms.v1, the whole of which is the help
// page `pipeline/rooms-protocol`: one WebSocket per joined room, the state
// kept in step with `joined`, `patch` and `resync`, the room's events handed
// to listeners, and a dropped connection taken back with the resume token
// while the server still holds the seat.
//
// Nothing here touches window, document, WebSocket or fetch until a room is
// joined, so the module is safe to import while a page renders on the server.
// The hooks are the Zeb React globals the page installs before this bundle
// loads, taken the way zeb/use takes them.

const { useState, useEffect, useRef, useCallback } = globalThis;

const PROTOCOL = "zebflow.rooms.v1";

// Where the page server writes this page's room base: an attribute on the
// element that already carries the page's state to the browser.
const PAGE_DATA_ID = "__rwe_payload";
const PAGE_ROOMS_ATTRIBUTE = "data-rooms";

// Close codes the server sends on purpose. None is retried: the room said no,
// or it ended this session (help `pipeline/rooms-protocol` § Close codes).
const REFUSED = new Set([
  1008, // too many events waiting: the client flooded the room
  4401, // the join rule, the ticket or the resume token refused
  4404, // the room is not open
  4408, // the login the join rule proved expired: the page signs in again
  4409, // the server closed this session (ws.session.close)
  4410, // the room was closed (ws.room.close)
  4429, // the room is full
]);
// A refused resume token means the seat is gone (the hold ended, or the
// token was used): one fresh join takes a new seat.
const TOKEN_REFUSED = 4401;
const NORMAL_CLOSURE = 1000;
const OPEN = 1;

// Reconnect backoff: the first wait, doubled per failed attempt, up to the cap;
// each wait is drawn from its upper half so clients do not return in step.
const FIRST_DELAY_MS = 250;
const MAX_DELAY_MS = 5000;

// ── State patches ──────────────────────────────────────────────────────────
// `set` and `del` by path, with structural sharing: only the objects and
// arrays along a changed path are new; everything else keeps its identity,
// so a `select` comparing with Object.is sees exactly what changed.

// Never a key: writing through one reaches the prototype chain.
const UNSAFE_KEYS = new Set(["__proto__", "constructor", "prototype"]);
const SKIP = Symbol("skip");

const isObject = (value) => value !== null && typeof value === "object";
const hasOwn = (object, key) => Object.prototype.hasOwnProperty.call(object, key);

function isSafePath(path) {
  return (
    Array.isArray(path) &&
    path.length > 0 &&
    path.every((key) =>
      typeof key === "string" ? !UNSAFE_KEYS.has(key) : Number.isInteger(key) && key >= 0,
    )
  );
}

// A copy of `node` that this frame may change; a copy this frame already made
// is changed in place, so two ops under one parent copy it once.
function writable(node, fresh) {
  if (fresh.has(node)) return node;
  const copy = Array.isArray(node) ? node.slice() : { ...node };
  fresh.add(copy);
  return copy;
}

// The key as `node` holds it: an array takes a plain number, an object a string.
function keyFor(node, key) {
  if (Array.isArray(node)) return typeof key === "number" ? key : SKIP;
  return String(key);
}

function setIn(node, path, at, value, fresh) {
  // A missing or non-object step becomes a new container.
  const base = isObject(node) ? node : typeof path[at] === "number" ? [] : {};
  const key = keyFor(base, path[at]);
  if (key === SKIP || (Array.isArray(base) && key > base.length)) return SKIP;
  const had = hasOwn(base, key);
  const child = had ? base[key] : undefined;
  const next = at === path.length - 1 ? value : setIn(child, path, at + 1, value, fresh);
  if (next === SKIP) return SKIP;
  if (had && Object.is(child, next)) return node;
  let out = base;
  if (base === node) out = writable(node, fresh);
  else fresh.add(base);
  out[key] = next;
  return out;
}

function deleteIn(node, path, at, fresh) {
  if (!isObject(node)) return node;
  const key = keyFor(node, path[at]);
  if (key === SKIP || !hasOwn(node, key)) return node;
  if (at === path.length - 1) {
    const out = writable(node, fresh);
    if (Array.isArray(out)) out.splice(key, 1);
    else delete out[key];
    return out;
  }
  const child = node[key];
  const next = deleteIn(child, path, at + 1, fresh);
  if (next === child) return node;
  const out = writable(node, fresh);
  out[key] = next;
  return out;
}

// Apply one frame's ops in order. An op with an unsafe or malformed path is
// skipped whole; the ops around it still apply.
function applyOps(state, ops) {
  const fresh = new WeakSet();
  let root = state;
  for (const op of ops) {
    if (!isObject(op) || !isSafePath(op.path)) continue;
    let next = root;
    if (op.op === "set") next = setIn(root, op.path, 0, op.value, fresh);
    else if (op.op === "del") next = deleteIn(root, op.path, 0, fresh);
    if (next !== SKIP) root = next;
  }
  return root;
}

// ── Where rooms live ───────────────────────────────────────────────────────

// The base URL a room name is appended to, absolute, ending in `/`; null when
// this page has no rooms (the project's `ws` surface is off).
function roomsBase(url) {
  let base = url;
  if (base == null) {
    const page = globalThis.document;
    const carrier = page && typeof page.getElementById === "function" ? page.getElementById(PAGE_DATA_ID) : null;
    base = carrier ? carrier.getAttribute(PAGE_ROOMS_ATTRIBUTE) : null;
  }
  if (typeof base !== "string" || base === "") return null;
  if (!base.endsWith("/")) base += "/";
  if (/^wss?:\/\//i.test(base)) return base;
  if (/^https?:\/\//i.test(base)) return "ws" + base.slice(4);
  const here = globalThis.location;
  if (!here || !here.host) return null;
  const scheme = here.protocol === "https:" ? "wss://" : "ws://";
  return scheme + here.host + (base.startsWith("/") ? base : "/" + base);
}

// ── Tickets ────────────────────────────────────────────────────────────────
// A string is a URL the ticket is POSTed for; a function returns a ticket or
// a promise of one; nothing means an open room. Fetched again for every fresh
// join: a ticket is usable once and expires within a minute.

class TicketFailure {
  constructor(status) {
    this.status = status;
  }
}

async function fetchTicket(source) {
  if (source == null) return null;
  if (typeof source === "function") {
    let ticket;
    try {
      ticket = await source();
    } catch {
      throw new TicketFailure(0);
    }
    if (typeof ticket !== "string" || ticket === "") throw new TicketFailure(0);
    return ticket;
  }
  if (typeof source !== "string") throw new TicketFailure(0);
  let response;
  try {
    response = await globalThis.fetch(source, {
      method: "POST",
      credentials: "same-origin",
      headers: { accept: "application/json" },
    });
  } catch {
    throw new TicketFailure(0);
  }
  if (!response.ok) throw new TicketFailure(response.status);
  let body;
  try {
    body = await response.json();
  } catch {
    throw new TicketFailure(response.status);
  }
  if (!isObject(body) || typeof body.ticket !== "string" || body.ticket === "") {
    throw new TicketFailure(response.status);
  }
  return body.ticket;
}

// A listener that throws must not stop the room: it is reported on its own.
function call(listener, ...args) {
  try {
    listener(...args);
  } catch (error) {
    queueMicrotask(() => {
      throw error;
    });
  }
}

// ── A joined room ──────────────────────────────────────────────────────────
// status: connecting → joined ⇄ away → closed. `away` is a dropped connection
// being taken back; `closed` is final and carries { code, reason }.

class Room {
  constructor(name, options) {
    this.name = name;
    this.state = undefined;
    this.status = "connecting";
    this.sessionId = null;
    this.closed = null;
    this._options = options;
    this._socket = null;
    this._resumeToken = null;
    this._resuming = false; // the current socket is a resume
    this._joined = false; // the current socket got `joined`
    this._seated = false; // joined at least once: a later drop is `away`
    this._attempt = 0;
    this._timer = null;
    this._generation = 0;
    this._listeners = new Map();
    this._stateListeners = new Set();
    this._statusListeners = new Set();
    // Next microtask, so listeners added right after joinRoom see every change.
    Promise.resolve().then(() => this._connect());
  }

  on(event, listener) {
    let listeners = this._listeners.get(event);
    if (!listeners) this._listeners.set(event, (listeners = new Set()));
    listeners.add(listener);
    return () => listeners.delete(listener);
  }

  onState(listener) {
    this._stateListeners.add(listener);
    return () => this._stateListeners.delete(listener);
  }

  onStatus(listener) {
    this._statusListeners.add(listener);
    return () => this._statusListeners.delete(listener);
  }

  // Sent now or not at all: false unless joined, and never queued. Names
  // starting with `$` are the server's own.
  send(event, payload) {
    const socket = this._socket;
    if (this.status !== "joined" || !socket || socket.readyState !== OPEN) return false;
    if (typeof event !== "string" || event === "" || event.startsWith("$")) return false;
    socket.send(JSON.stringify(payload === undefined ? { event } : { event, payload }));
    return true;
  }

  // Closes normally — the server gives the seat up at once — and never reconnects.
  leave() {
    this._close(NORMAL_CLOSURE, "left");
  }

  async _connect() {
    if (this.status === "closed") return;
    const generation = ++this._generation;
    const base = roomsBase(this._options.url);
    if (!base) return this._close(null, "rooms-off");
    let query = "";
    this._resuming = this._resumeToken !== null;
    if (this._resuming) {
      // A resume never fetches a ticket: the token is the pass.
      query = "?resume=" + encodeURIComponent(this._resumeToken);
    } else {
      let ticket;
      try {
        ticket = await fetchTicket(this._options.ticket);
      } catch (failure) {
        if (generation === this._generation) this._close(null, "ticket", { status: failure.status ?? 0 });
        return;
      }
      if (generation !== this._generation) return;
      if (ticket !== null) query = "?ticket=" + encodeURIComponent(ticket);
    }
    let socket;
    try {
      socket = new globalThis.WebSocket(base + encodeURIComponent(this.name) + query, PROTOCOL);
    } catch {
      // No WebSocket here, or a base that is not a URL: retrying cannot help.
      return this._close(null, "socket");
    }
    this._socket = socket;
    this._joined = false;
    socket.onmessage = (message) => {
      if (this._socket === socket) this._frame(message.data);
    };
    socket.onclose = (closing) => {
      if (this._socket === socket) this._dropped(closing.code, closing.reason);
    };
  }

  _frame(data) {
    let frame;
    try {
      frame = JSON.parse(data);
    } catch {
      return;
    }
    if (!isObject(frame)) return;
    switch (frame.type) {
      case "joined":
        this._joined = true;
        this._seated = true;
        this._attempt = 0;
        this.sessionId = typeof frame.session_id === "string" ? frame.session_id : null;
        this._resumeToken = typeof frame.resume === "string" && frame.resume !== "" ? frame.resume : null;
        this._replaceState(frame.state);
        this._setStatus("joined");
        return;
      case "resync":
        this._replaceState(frame.state);
        return;
      case "patch":
        // A tick's ops first, then that tick's events, in order.
        if (Array.isArray(frame.ops) && frame.ops.length > 0) {
          this.state = applyOps(this.state, frame.ops);
          for (const listener of [...this._stateListeners]) call(listener, this.state, frame.ops);
        }
        if (Array.isArray(frame.events)) {
          for (const sent of frame.events) {
            if (isObject(sent) && typeof sent.event === "string") this._emit(sent.event, sent.payload);
          }
        }
        return;
      case "event":
        if (typeof frame.event === "string") this._emit(frame.event, frame.payload);
        return;
      default:
        return; // a frame type this version does not know
    }
  }

  _replaceState(state) {
    this.state = state;
    for (const listener of [...this._stateListeners]) call(listener, this.state, null);
  }

  _emit(event, payload) {
    const listeners = this._listeners.get(event);
    if (listeners) for (const listener of [...listeners]) call(listener, payload);
    const onEvent = this._options.onEvent;
    if (typeof onEvent === "function") call(onEvent, event, payload);
  }

  _dropped(code, reason) {
    this._socket = null;
    if (this.status === "closed") return;
    if (REFUSED.has(code)) {
      if (code === TOKEN_REFUSED && this._resuming && !this._joined) {
        this._resumeToken = null;
        return this._connect();
      }
      return this._close(code, reason);
    }
    if (this._seated) this._setStatus("away");
    const ceiling = Math.min(MAX_DELAY_MS, FIRST_DELAY_MS * 2 ** this._attempt);
    this._attempt += 1;
    const wait = ceiling / 2 + (Math.random() * ceiling) / 2;
    this._timer = setTimeout(() => {
      this._timer = null;
      this._connect();
    }, wait);
  }

  _close(code, reason, extra) {
    if (this.status === "closed") return;
    this._generation += 1;
    if (this._timer !== null) clearTimeout(this._timer);
    this._timer = null;
    const socket = this._socket;
    this._socket = null;
    if (socket) {
      try {
        socket.close(NORMAL_CLOSURE);
      } catch {
        // already closing
      }
    }
    this.closed = { code, reason, ...extra };
    this._setStatus("closed", this.closed);
  }

  _setStatus(status, detail) {
    if (this.status === status) return;
    this.status = status;
    for (const listener of [...this._statusListeners]) call(listener, status, detail);
  }
}

/**
 * Join a room. Returns at once; the socket opens on the next microtask.
 *
 * options.ticket  — a URL to POST for `{ ticket }`, a function returning a
 *                   ticket (or a promise of one), or nothing for an open room.
 * options.url     — the rooms base for a client outside a Zebflow page,
 *                   e.g. "wss://play.example.com/_ws/rooms".
 * options.onEvent — (event, payload) for every event the room sends.
 */
export function joinRoom(name, options) {
  return new Room(String(name), options || {});
}

/**
 * A room for a component: joined in an effect, left on unmount or when
 * `name` changes. Re-renders on status changes and on state changes that
 * `options.select` (when given) says matter.
 */
export function useRoom(name, options) {
  const latest = useRef(options || {});
  latest.current = options || {};
  const current = useRef(null);
  const [, rerender] = useState(0);

  useEffect(() => {
    const pick = (state) => {
      const select = latest.current.select;
      return state === undefined || typeof select !== "function" ? state : select(state);
    };
    const room = joinRoom(name, {
      url: latest.current.url,
      get ticket() {
        return latest.current.ticket;
      },
      onEvent: (event, payload) => {
        const onEvent = latest.current.onEvent;
        if (typeof onEvent === "function") onEvent(event, payload);
      },
    });
    current.current = room;
    let shown = pick(room.state);
    const offState = room.onState((state) => {
      const next = pick(state);
      if (Object.is(shown, next)) return;
      shown = next;
      rerender((count) => count + 1);
    });
    const offStatus = room.onStatus(() => rerender((count) => count + 1));
    rerender((count) => count + 1);
    return () => {
      offState();
      offStatus();
      room.leave();
      if (current.current === room) current.current = null;
    };
  }, [name]);

  const send = useCallback((event, payload) => {
    const room = current.current;
    return room ? room.send(event, payload) : false;
  }, []);

  const room = current.current && current.current.name === String(name) ? current.current : null;
  if (!room) return { state: undefined, status: "connecting", send, room: null };
  const select = latest.current.select;
  const state = room.state === undefined || typeof select !== "function" ? room.state : select(room.state);
  return { state, status: room.status, send, room };
}
