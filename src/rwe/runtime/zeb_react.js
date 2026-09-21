// Zeb React — Zebflow's dependency-free function-component runtime (MIT).
// This exact source runs in the browser and in the embedded V8 SSR workers.
(function () {
  'use strict';
  if (globalThis.__zebReact) return;

  const Fragment = Symbol.for('zeb.fragment');
  const ErrorBoundary = Symbol.for('zeb.error-boundary');
  const Portal = Symbol.for('zeb.portal');
  const Text = Symbol('text');
  const Empty = Symbol('empty');
  const roots = new WeakMap();
  let active = null;
  let hookIndex = 0;
  const voidTags = new Set('area base br col embed hr img input link meta param source track wbr'.split(' '));
  const booleanAttrs = new Set('allowFullScreen async autoFocus autoPlay checked controls default defer disabled disablePictureInPicture formNoValidate hidden inert isMap itemScope loop multiple muted noModule noValidate open playsInline readOnly required reversed scoped seamless selected'.toLowerCase().split(' '));
  const unitless = /^(animationIterationCount|aspectRatio|borderImageOutset|borderImageSlice|borderImageWidth|columnCount|fillOpacity|flex|flexGrow|flexShrink|fontWeight|gridArea|gridColumn|gridColumnEnd|gridColumnStart|gridRow|gridRowEnd|gridRowStart|lineHeight|opacity|order|orphans|scale|stopOpacity|strokeDasharray|strokeDashoffset|strokeMiterlimit|strokeOpacity|strokeWidth|tabSize|widows|zIndex|zoom)$/;
  const svgNS = 'http://www.w3.org/2000/svg';

  function h(type, props, ...children) {
    props = { ...props };
    if (children.length) props.children = children.length === 1 ? children[0] : children;
    if (type && type.defaultProps) {
      for (const key in type.defaultProps) if (props[key] === undefined) props[key] = type.defaultProps[key];
    }
    const key = props.key == null ? null : props.key;
    delete props.key;
    return { type, props, key };
  }
  function jsx(type, props, key) { return h(type, key === undefined ? props : { ...props, key }); }
  function normalize(value) {
    if (value == null || typeof value === 'boolean') return { type: Empty, props: {}, key: null };
    if (Array.isArray(value)) return h(Fragment, null, value);
    if (['string', 'number', 'bigint'].includes(typeof value)) return { type: Text, props: { value: String(value) }, key: null };
    if (typeof value === 'object' && value.type != null) return value;
    throw new TypeError('Zeb React: invalid child');
  }
  function children(value, out = []) {
    if (Array.isArray(value)) for (const item of value) children(item, out);
    else out.push(normalize(value));
    return out;
  }
  function createContext(value) {
    const context = { defaultValue: value };
    context.Provider = function Provider(p) { return p.children; };
    context.Provider.context = context;
    context.Consumer = function Consumer(p) { return p.children(useContext(context)); };
    return context;
  }
  function slot(kind) {
    if (!active) throw new Error('Zeb React: hooks must run inside a component');
    const index = hookIndex++;
    const previous = active.hooks[index];
    if (previous && previous.kind !== kind) throw new Error('Zeb React: hook order changed');
    return active.hooks[index] || (active.hooks[index] = { kind, owner: active });
  }
  function changed(a, b) {
    return !a || !b || a.length !== b.length || a.some((v, i) => !Object.is(v, b[i]));
  }
  function schedule(owner) {
    if (!owner.mounted || owner.root.ssr || !owner.root.mounted) return;
    owner.selfDirty = true;
    for (let node = owner; node; node = node.parent) node.dirty = true;
    const root = owner.root;
    if (!root.scheduled) {
      root.scheduled = true;
      queueMicrotask(() => { if (root.scheduled) updateRoot(root); });
    }
  }
  function useReducer(reducer, initial, init) {
    const s = slot('state');
    s.reducer = reducer;
    if (!s.dispatch) {
      s.value = init ? init(initial) : initial;
      s.dispatch = value => {
        if (!s.owner.mounted || s.owner.root.ssr) return;
        const next = s.reducer(s.value, value);
        if (!Object.is(next, s.value)) { s.value = next; schedule(s.owner); }
      };
    }
    return [s.value, s.dispatch];
  }
  function useState(initial) {
    return useReducer((old, value) => typeof value === 'function' ? value(old) : value,
      initial, value => typeof value === 'function' ? value() : value);
  }
  function readSnapshot(getSnapshot) {
    const value = getSnapshot();
    if (!Object.is(value, getSnapshot())) throw new Error('Zeb React: getSnapshot must return a cached snapshot');
    return value;
  }
  function useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot) {
    const s = slot('store'), root = active.root;
    const server = root.ssr || root.hydrating;
    if (typeof subscribe !== 'function' || typeof getSnapshot !== 'function') {
      throw new TypeError('Zeb React: useSyncExternalStore requires subscribe and getSnapshot functions');
    }
    if (server && typeof getServerSnapshot !== 'function') {
      throw new Error('Zeb React: useSyncExternalStore requires getServerSnapshot for SSR and hydration');
    }
    s.value = readSnapshot(server ? getServerSnapshot : getSnapshot);
    s.getSnapshot = getSnapshot; s.subscribe = subscribe;
    if (!root.ssr) { root.stores.add(s); root.subscriptions.add(s); }
    return s.value;
  }
  function checkStore(s) {
    if (!s.owner.mounted) return;
    try { if (Object.is(s.value, readSnapshot(s.getSnapshot))) return; }
    catch (_) { /* Re-read during rendering so a boundary receives the error. */ }
    schedule(s.owner);
  }
  function flushSubscriptions(root) {
    const pending = Array.from(root.subscriptions); root.subscriptions.clear();
    for (const s of pending) {
      if (!s.owner.mounted || hasFailedAncestor(s.owner)) continue;
      guard(s.owner, () => {
        if (s.connected !== s.subscribe) {
          const cleanup = s.cleanup; s.cleanup = undefined; s.connected = undefined;
          if (cleanup) cleanup();
          let listening = true;
          const unsubscribe = s.subscribe(() => {
            if (!listening || !s.owner.mounted) return;
            checkStore(s);
            if (root.scheduled && !root.working) updateRoot(root);
          });
          if (typeof unsubscribe !== 'function') throw new TypeError('Zeb React: subscribe must return an unsubscribe function');
          s.connected = s.subscribe;
          s.cleanup = () => { listening = false; unsubscribe(); };
        }
        // Covers changes between rendering and subscribing, including a
        // subscribe implementation that changes the store without notifying.
        checkStore(s);
      });
    }
  }
  function useRef(initial) {
    const s = slot('ref');
    return s.value || (s.value = { current: initial });
  }
  function useMemo(factory, deps) {
    const s = slot('memo');
    if (!s.ready || changed(s.deps, deps)) {
      s.value = factory(); s.deps = deps && deps.slice(); s.ready = true;
    }
    return s.value;
  }
  function useCallback(fn, deps) { return useMemo(() => fn, deps); }
  function useContext(context) {
    slot('context');
    return active.context.has(context) ? active.context.get(context) : context.defaultValue;
  }
  function useId() {
    const s = slot('id');
    return s.value || (s.value = 'zeb-' + active.root.nextId++);
  }
  function effect(kind, fn, deps) {
    const s = slot(kind);
    if (active.root.ssr) return;
    if (!s.ready || changed(s.deps, deps)) {
      s.fn = fn; s.deps = deps && deps.slice(); s.ready = true;
      active.root[kind].add(s);
    }
  }
  function useEffect(fn, deps) { effect('effects', fn, deps); }
  function useLayoutEffect(fn, deps) { effect('layouts', fn, deps); }
  function setRef(ref, value) {
    if (typeof ref === 'function') ref(value);
    else if (ref) ref.current = value;
  }
  function useImperativeHandle(ref, factory, deps) {
    useLayoutEffect(() => { setRef(ref, factory()); return () => setRef(ref, null); },
      deps && [...deps, ref]);
  }
  function forwardRef(fn) { return function ForwardRef(props) { return fn(props, props.ref || null); }; }
  function shallowEqual(a, b) {
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every(k => Object.hasOwn(b, k) && Object.is(a[k], b[k]));
  }
  function memo(component, compare = shallowEqual) {
    function Memo(props) { return component(props); }
    Memo.compare = compare;
    return Memo;
  }
  function createPortal(value, container, key) { return h(Portal, { children: value, container, key }); }
  class RenderFailure {
    constructor(error, source) { this.error = error; this.source = source; }
  }
  function failure(error, source) { return error instanceof RenderFailure ? error : new RenderFailure(error, source); }
  function componentInfo(source) {
    let componentStack = '';
    for (let node = source; node; node = node.parent) {
      const type = node.type;
      if (typeof type === 'function') componentStack += '\n    at ' + (type.displayName || type.name || 'Anonymous');
      else if (type === ErrorBoundary) componentStack += '\n    at ErrorBoundary';
    }
    return { componentStack };
  }
  function hasFailedAncestor(owner) {
    for (let node = owner.parent; node; node = node.parent) if (node.failure && !node.showingFallback) return true;
    return false;
  }
  function guard(owner, fn) {
    try { return fn(); }
    catch (error) {
      for (let node = owner.parent; node; node = node.parent) {
        if (node.type === ErrorBoundary && node.mounted && !node.failure && !node.showingFallback) {
          node.failure = failure(error, owner); schedule(node); return;
        }
      }
      owner.root.errors.push(failure(error, owner));
    }
  }
  function boundaryFallback(props, error, resetErrorBoundary) {
    if (typeof props.fallbackRender === 'function') return props.fallbackRender({ error, resetErrorBoundary });
    if (Object.hasOwn(props, 'fallback')) return props.fallback;
    throw error;
  }
  function resetBoundary(instance, details) {
    if (!instance.mounted || !instance.failure) return;
    // onReset failures belong to an ancestor, never to this boundary.
    guard(instance, () => {
      if (instance.props.onReset) instance.props.onReset(details);
      instance.failure = null; instance.notified = false; instance.needsReset = true;
      schedule(instance);
    });
  }
  function reconcileBoundary(parentDom, instance, context, svg, claim) {
    const props = instance.props;
    if (!instance.reset) instance.reset = (...args) => resetBoundary(instance, { reason: 'imperative-api', args });
    if (instance.failure && instance.notified && changed(instance.resetKeys || [], props.resetKeys || [])) {
      resetBoundary(instance, { reason: 'keys', prev: instance.resetKeys, next: props.resetKeys });
    }
    instance.resetKeys = props.resetKeys && props.resetKeys.slice();
    if (instance.needsReset) {
      for (const child of instance.children) dispose(child);
      instance.children = []; instance.needsReset = false; instance.showingFallback = false;
    }
    if (!instance.failure) {
      const nextId = instance.root.nextId;
      try { reconcileChildren(parentDom, instance, props.children, context, svg, claim); return; }
      catch (error) {
        instance.failure = failure(error, instance);
        // Failed initial attempts must not consume the fallback's SSR IDs.
        instance.root.nextId = nextId;
      }
    }
    if (!instance.notified) {
      instance.notified = true;
      if (props.onError) props.onError(instance.failure.error, componentInfo(instance.failure.source));
      for (const child of instance.children) dispose(child);
      instance.children = [];
    }
    instance.showingFallback = true;
    // Outside the try block: a broken fallback belongs to the next boundary.
    reconcileChildren(parentDom, instance,
      boundaryFallback(props, instance.failure.error, instance.reset), context, svg, claim);
  }
  function invoke(instance, props) {
    const previous = active, previousIndex = hookIndex;
    active = instance; hookIndex = 0;
    try {
      const result = instance.type(props);
      if (instance.hookCount != null && instance.hookCount !== hookIndex) throw new Error('Zeb React: hook count changed');
      instance.hookCount = hookIndex;
      return result;
    } finally { active = previous; hookIndex = previousIndex; }
  }
  function flushEffects(queue) {
    const pending = Array.from(queue);
    queue.clear();
    for (const s of pending) {
      if (!s.owner.mounted || hasFailedAncestor(s.owner)) continue;
      const cleanup = s.cleanup;
      s.cleanup = undefined;
      guard(s.owner, () => {
        if (typeof cleanup === 'function') cleanup();
        s.cleanup = s.fn();
      });
    }
  }
  function cssName(name) {
    return name.startsWith('--') ? name : name.replace(/[A-Z]/g, m => '-' + m.toLowerCase()).replace(/^ms-/, '-ms-');
  }
  function cssValue(name, value) {
    return typeof value === 'number' && value !== 0 && !name.startsWith('--') && !unitless.test(name.replace(/^(Webkit|Moz|ms|O)(?=[A-Z])/, '').replace(/^./, c => c.toLowerCase())) ? value + 'px' : String(value);
  }
  function styleText(style) {
    if (typeof style !== 'object' || style === null) return style || '';
    return Object.keys(style).filter(k => style[k] != null && style[k] !== false)
      .map(k => cssName(k) + ':' + cssValue(k, style[k])).join(';');
  }
  function attrName(name, svg) {
    if (name === 'className') return 'class';
    if (name === 'htmlFor') return 'for';
    if (name === 'defaultValue') return 'value';
    if (name === 'defaultChecked') return 'checked';
    if (name === 'xlinkHref') return 'xlink:href';
    if (svg && /^(stroke|fill|clip|font|text|paint|color|dominant|alignment|stop|flood|marker)[A-Z]/.test(name)) return cssName(name);
    if (name === 'tabIndex') return 'tabindex';
    return name;
  }
  function ignored(name) { return ['children', 'key', 'ref', 'dangerouslySetInnerHTML', 'suppressHydrationWarning'].includes(name); }
  function attrValue(name, value) {
    if (value == null || typeof value === 'function' || typeof value === 'symbol') return null;
    if (booleanAttrs.has(name.toLowerCase())) return value ? '' : null;
    return String(value);
  }
  function setProperty(dom, name, value, old, svg, hydrating) {
    if (ignored(name)) return;
    if (/^on[A-Za-z]/.test(name)) {
      let event = name.slice(2), capture = event.endsWith('Capture');
      if (capture) event = event.slice(0, -7);
      const lower = event.toLowerCase();
      event = ('on' + lower in dom) ? lower : event;
      if (lower === 'doubleclick') event = 'dblclick';
      const id = event + ':' + capture;
      const listeners = dom.__zebListeners || (dom.__zebListeners = new Map());
      const previous = listeners.get(id);
      if (previous) dom.removeEventListener(event, previous, capture);
      if (typeof value === 'function') { dom.addEventListener(event, value, capture); listeners.set(id, value); }
      else listeners.delete(id);
      return;
    }
    if (name === 'style') { dom.style.cssText = styleText(value); return; }
    if (!svg && (name === 'value' || name === 'checked' || name === 'selected')) {
      // Preserve edits made between receiving SSR HTML and attaching handlers.
      // `selected` is here for the same reason with one more: the server marks
      // an <option> selected from its <select>'s value or defaultValue, and the
      // option's own props never carry it — treating that attribute as stale
      // on hydration dropped it, and every server-chosen select snapped back
      // to its first option the moment the page hydrated.
      if (hydrating) return;
      const next = name === 'checked' ? !!value : value == null ? '' : String(value);
      if (dom[name] !== next) dom[name] = next;
      return;
    }
    if (!svg && (name === 'defaultValue' || name === 'defaultChecked')) {
      if (old === undefined && !hydrating) dom[name] = value == null ? '' : value;
      return;
    }
    const attr = attrName(name, svg), serialized = attrValue(attr, value);
    if (serialized === null) dom.removeAttribute(attr);
    else dom.setAttribute(attr, serialized);
    if (!svg && name === 'muted') dom.muted = !!value;
  }
  function patchProps(dom, props, previous, svg, hydrating) {
    for (const name of Object.keys(previous)) if (!(name in props)) setProperty(dom, name, null, previous[name], svg, hydrating);
    for (const name of Object.keys(props)) {
      if (!Object.is(props[name], previous[name]) || name === 'value' || name === 'checked') {
        setProperty(dom, name, props[name], previous[name], svg, hydrating);
      }
    }
  }
  function nodes(instance, out = []) {
    if (instance.dom) out.push(instance.dom);
    else if (instance.type !== Portal) for (const child of instance.children) nodes(child, out);
    return out;
  }
  function dispose(instance) {
    if (!instance.mounted) return;
    instance.mounted = false;
    for (const s of instance.hooks) {
      instance.root.effects.delete(s); instance.root.layouts.delete(s);
      instance.root.stores.delete(s); instance.root.subscriptions.delete(s);
      if (typeof s.cleanup === 'function') { const cleanup = s.cleanup; s.cleanup = undefined; guard(instance, cleanup); }
    }
    for (const child of instance.children) dispose(child);
    if (instance.dom) {
      guard(instance, () => setRef(instance.props.ref, null));
      instance.dom.remove();
    }
  }
  function claimNode(claim, predicate) {
    if (claim && claim.next && predicate(claim.next)) {
      const dom = claim.next;
      claim.next = dom.nextSibling;
      return dom;
    }
    return null;
  }
  function cleanClaim(claim) {
    while (claim && claim.next) { const node = claim.next; claim.next = node.nextSibling; node.remove(); }
  }
  function reconcileChildren(parentDom, instance, values, context, svg, claim) {
    const old = instance.children, next = [], used = new Set(), keyed = new Map();
    for (const child of old) if (child.key != null) keyed.set(child.key, child);
    const list = children(values);
    // Own partial work immediately, so a boundary can dispose nodes, portals,
    // hooks and subscriptions even when a later sibling fails to render.
    instance.children = next;
    try {
      for (let index = 0; index < list.length; index++) {
        const vnode = list[index];
        let previous = vnode.key != null ? keyed.get(vnode.key) : old[index];
        if (previous && (used.has(previous) || previous.key !== vnode.key || previous.type !== vnode.type)) previous = null;
        if (previous) used.add(previous);
        const child = previous || { type: vnode.type, key: vnode.key, props: {}, children: [], hooks: [], root: instance.root, parent: instance, mounted: true, dirty: false };
        next.push(child);
        reconcile(parentDom, vnode, child, context, svg, claim);
      }
    } catch (error) {
      for (const child of old) if (!used.has(child)) next.push(child);
      throw error;
    }
    for (const child of old) if (!used.has(child)) dispose(child);
    instance.children = next;
  }
  function orderChildren(parentDom, list) {
    // Only move managed nodes; imperative library children remain untouched.
    const ordered = list.flatMap(child => nodes(child));
    let cursor = parentDom.firstChild;
    for (const node of ordered) {
      if (node !== cursor) parentDom.insertBefore(node, cursor);
      cursor = node.nextSibling;
    }
  }
  function reconcile(parentDom, vnode, instance, context, svg, claim) {
    try { return reconcileInstance(parentDom, vnode, instance, context, svg, claim); }
    catch (error) { throw failure(error, instance); }
  }
  function reconcileInstance(parentDom, vnode, instance, context, svg, claim) {
    const fresh = !instance.initialized;
    instance.initialized = true;
    const type = vnode.type, props = vnode.props, previous = instance.props;
    const contextChanged = !instance.context || instance.context.size !== context.size || [...context].some(([k, v]) => !Object.is(instance.context.get(k), v));
    instance.context = context;
    const reusable = !fresh && !instance.selfDirty && !contextChanged && previous.ref === props.ref &&
      (previous === props || (typeof type === 'function' && type.compare && type.compare(previous, props)));
    if (reusable && !instance.dirty && typeof type === 'function') return instance;
    instance.props = props; instance.dirty = false; instance.selfDirty = false;
    if (type === Text || type === Empty) {
      if (fresh) {
        instance.dom = claimNode(claim, node => type === Text ? node.nodeType === 3 : node.nodeType === 8 && node.data === 'zeb');
        if (!instance.dom) instance.dom = type === Text ? parentDom.ownerDocument.createTextNode(props.value) : parentDom.ownerDocument.createComment('zeb');
        // HTML parsers coalesce adjacent text nodes; split to preserve each vnode.
        if (type === Text && claim && instance.dom.data.startsWith(props.value) && instance.dom.data.length > props.value.length && props.value.length) {
          claim.next = instance.dom.splitText(props.value.length);
        }
      }
      if (type === Text && instance.dom.data !== props.value) instance.dom.data = props.value;
    } else if (type === Portal) {
      if (previous.container && previous.container !== props.container) {
        for (const child of instance.children) dispose(child);
        instance.children = [];
      }
      reconcileChildren(props.container, instance, props.children, context, false, null);
      orderChildren(props.container, instance.children);
    } else if (type === ErrorBoundary) {
      reconcileBoundary(parentDom, instance, context, svg, claim);
    } else if (type === Fragment || typeof type === 'function') {
      let childContext = context;
      if (type.context) { childContext = new Map(context); childContext.set(type.context, props.value); }
      const result = type === Fragment ? props.children : reusable ? instance.output : invoke(instance, props);
      instance.output = result;
      reconcileChildren(parentDom, instance, result, childContext, svg, claim);
    } else if (typeof type === 'string') {
      svg = type === 'svg' || svg;
      if (fresh) instance.dom = claimNode(claim, node => node.nodeType === 1 && node.localName === type && (node.namespaceURI === svgNS) === svg)
        || (svg ? parentDom.ownerDocument.createElementNS(svgNS, type) : parentDom.ownerDocument.createElement(type, props.is ? { is: props.is } : undefined));
      const dom = instance.dom;
      const hydration = fresh && !!claim && dom.parentNode != null;
      const oldProps = hydration ? Object.fromEntries(Array.from(dom.attributes, attr => [attr.name, attr.value])) : previous;
      patchProps(dom, props, oldProps, svg, hydration);
      if (props.dangerouslySetInnerHTML != null) {
        for (const child of instance.children) dispose(child);
        instance.children = [];
        const html = String(props.dangerouslySetInnerHTML.__html ?? '');
        if (dom.innerHTML !== html) dom.innerHTML = html;
      } else if (!voidTags.has(type)) {
        if (previous.dangerouslySetInnerHTML != null) dom.textContent = '';
        const childClaim = hydration ? { next: dom.firstChild } : null;
        const value = type === 'textarea' ? (props.value ?? props.defaultValue ?? props.children) : props.children;
        reconcileChildren(dom, instance, value, context, svg && type !== 'foreignObject', childClaim);
        cleanClaim(childClaim);
        orderChildren(dom, instance.children);
      }
      // Select values must be applied after options exist.
      if (!hydration && 'value' in props) {
        if (type === 'select' && props.multiple && Array.isArray(props.value)) {
          for (const option of dom.options) option.selected = props.value.some(v => String(v) === option.value);
        } else setProperty(dom, 'value', props.value, previous.value, svg, false);
      }
      if (previous.ref !== props.ref) {
        if (previous.ref) instance.root.refs.push({ owner: instance, attach: () => setRef(previous.ref, null) });
        if (props.ref) instance.root.refs.push({ owner: instance, attach: () => setRef(props.ref, dom) });
      }
    } else throw new TypeError('Zeb React: unsupported component type');
    return instance;
  }
  function updateRoot(root, claim = null) {
    if (root.working || !root.mounted) return;
    root.working = true;
    try {
      flushEffects(root.effects);
      let passes = 0;
      do {
        if (++passes > 50) throw new Error('Zeb React: too many synchronous updates');
        root.scheduled = false;
        root.hydrating = !!claim;
        if (!root.hydrating) for (const s of root.stores) checkStore(s);
        // All subscribers are checked before rendering, even if only one store
        // listener fired. Memoized siblings see the same snapshot in this commit.
        root.scheduled = false;
        const document = root.container.ownerDocument;
        const focused = document.activeElement;
        const selection = focused && typeof focused.selectionStart === 'number'
          ? [focused.selectionStart, focused.selectionEnd, focused.selectionDirection] : null;
        reconcileChildren(root.container, root, root.value, new Map(), false, claim);
        cleanClaim(claim);
        claim = null; root.hydrating = false;
        orderChildren(root.container, root.children);
        if (focused && focused.isConnected && document.activeElement !== focused && document.activeElement === document.body) {
          focused.focus({ preventScroll: true });
          if (selection) focused.setSelectionRange(...selection);
        }
        const refs = root.refs.splice(0);
        for (const { owner, attach } of refs) if (owner.mounted && !hasFailedAncestor(owner)) guard(owner, attach);
        flushSubscriptions(root);
        flushEffects(root.layouts);
      } while (root.scheduled);
      if (root.errors.length) throw root.errors.shift();
      if (root.effects.size && !root.effectTimer) root.effectTimer = setTimeout(() => {
        root.effectTimer = null; root.working = true;
        try { flushEffects(root.effects); }
        finally { root.working = false; }
        if (root.scheduled) updateRoot(root);
        if (root.errors.length) throw root.errors.shift().error;
      }, 0);
    } catch (error) {
      root.scheduled = false;
      // An uncaught failure must not leave partially mounted work behind.
      for (const child of root.children) dispose(child);
      root.children = []; root.refs = []; root.errors = []; root.scheduled = false;
      clearTimeout(root.effectTimer); root.effectTimer = null;
      throw error instanceof RenderFailure ? error.error : error;
    } finally { root.working = false; root.hydrating = false; }
  }
  function mount(value, container, hydrating) {
    let root = roots.get(container);
    if (value == null) {
      if (root) {
        root.scheduled = false; root.mounted = false;
        for (const child of root.children) dispose(child);
        clearTimeout(root.effectTimer);
        root.effects.clear(); root.layouts.clear(); root.refs = [];
        roots.delete(container);
        if (root.errors.length) throw root.errors.shift().error;
      }
      return;
    }
    if (!root) {
      root = { container, children: [], hooks: [], nextId: 0, effects: new Set(), layouts: new Set(), stores: new Set(), subscriptions: new Set(), refs: [], errors: [], value, mounted: true };
      root.root = root;
      roots.set(container, root);
      if (!hydrating) container.textContent = '';
    }
    root.value = value;
    updateRoot(root, hydrating && root.children.length === 0 ? { next: container.firstChild } : null);
  }
  function render(value, container) { mount(value, container, false); }
  function hydrate(value, container) { mount(value, container, true); }

  function escape(value) {
    return String(value).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function renderToString(value, options = {}) {
    const root = { ssr: true, nextId: 0 };
    function visit(value, context, svg, selected, parent = null, boundaryDepth = 0) {
      const vnode = normalize(value), { type, props } = vnode;
      if (type === Empty) return '';
      if (type === Text) return escape(props.value);
      if (type === Portal) return '';
      if (type === Fragment) return children(props.children).map(v => visit(v, context, svg, selected, parent, boundaryDepth)).join('');
      if (type === ErrorBoundary) {
        const instance = { type, parent }, nextId = root.nextId;
        try { return visit(props.children, context, svg, selected, instance, boundaryDepth + 1); }
        catch (error) {
          const caught = failure(error, instance);
          root.nextId = nextId;
          if (props.onError) props.onError(caught.error, componentInfo(caught.source));
          return visit(boundaryFallback(props, caught.error, () => {}), context, svg, selected, instance, boundaryDepth);
        }
      }
      if (typeof type === 'function') {
        let childContext = context;
        if (type.context) { childContext = new Map(context); childContext.set(type.context, props.value); }
        const instance = { type, hooks: [], context, root, mounted: true, parent };
        try { return visit(invoke(instance, props), childContext, svg, selected, instance, boundaryDepth); }
        catch (error) {
          const caught = failure(error, instance);
          if (!boundaryDepth && options.onError) return options.onError(caught.error);
          throw caught;
        }
      }
      if (typeof type !== 'string' || !/^[a-zA-Z][a-zA-Z0-9:._-]*$/.test(type)) throw new TypeError('Zeb React: invalid tag');
      svg = svg || type === 'svg';
      let attrs = '';
      const domProps = { ...props };
      if (type === 'select') selected = props.value ?? props.defaultValue;
      if (type === 'option' && selected != null) domProps.selected = (Array.isArray(selected) ? selected : [selected]).some(v => String(v) === String(props.value ?? props.children));
      for (const name of Object.keys(domProps)) {
        if (ignored(name) || /^on/i.test(name) || ((type === 'textarea' || type === 'select') && (name === 'value' || name === 'defaultValue'))) continue;
        const attr = attrName(name, svg);
        if (!/^[^\s"'<>/=]+$/.test(attr)) throw new TypeError('Zeb React: invalid attribute');
        const value = name === 'style' ? styleText(domProps[name]) : attrValue(attr, domProps[name]);
        if (value != null) attrs += value === '' ? ' ' + attr + '=""' : ' ' + attr + '="' + escape(value) + '"';
      }
      if (voidTags.has(type)) return '<' + type + attrs + '>';
      const content = type === 'textarea' ? (props.value ?? props.defaultValue ?? props.children) : props.children;
      let inner = props.dangerouslySetInnerHTML != null ? String(props.dangerouslySetInnerHTML.__html ?? '')
        : children(content).map(v => visit(v, context, svg && type !== 'foreignObject', selected, parent, boundaryDepth)).join('');
      if ((type === 'textarea' || type === 'pre' || type === 'listing') && inner.startsWith('\n')) inner = '\n' + inner;
      return '<' + type + attrs + '>' + inner + '</' + type + '>';
    }
    try { return visit(value, new Map(), false, undefined); }
    catch (error) { throw error instanceof RenderFailure ? error.error : error; }
  }

  globalThis.__zebReact = {
    h, createElement: h, Fragment, ErrorBoundary, jsx, jsxs: jsx, jsxDEV: jsx,
    render, hydrate, renderToString, createContext, createPortal, forwardRef, memo,
    useState, useReducer, useSyncExternalStore, useRef, useMemo, useCallback, useContext, useId,
    useEffect, useLayoutEffect, useImperativeHandle,
  };
})();
