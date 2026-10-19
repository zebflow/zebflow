// zeb_ssr_libs.js — server-side stand-ins for the runtime `zeb/*` libraries.
//
// Loaded once into every V8 worker, after zeb_ssr_init.js. A page that writes
// `import { ThreeScene } from "zeb/threejs"` is compiled to read its names from
// `globalThis.__zebLibs["zeb/threejs"]` (src/rwe/core/zeb_lib.rs). In the browser
// the loader files the real bundle under that key; on the server this file files
// a stand-in under the same key. Nothing here is installed as a global, so two
// libraries may export one name and neither replaces the other.
//
// Every runtime library has an entry, even those with nothing to draw on the
// server (zeb/pdf, zeb/prosemirror, …): naming an export of a library without
// a stand-in binds `undefined` and fails where it is used, not at the import.
// Components render the same placeholder markup the browser bundle expects to
// find, so hydration matches.

(function () {
  "use strict";

  var { h, createContext, useRef, useEffect } = globalThis.__zebReact;
  var libs = (globalThis.__zebLibs = globalThis.__zebLibs || {});

  function base64EncodeUtf8(text) {
    if (typeof btoa !== "undefined") {
      return btoa(unescape(encodeURIComponent(text)));
    }
    if (typeof Buffer !== "undefined") {
      return Buffer.from(text, "utf-8").toString("base64");
    }
    var encodedText = encodeURIComponent(String(text || ""));
    var bytes = [];
    for (var j = 0; j < encodedText.length; j++) {
      var ch = encodedText.charAt(j);
      if (ch === "%") {
        bytes.push(parseInt(encodedText.slice(j + 1, j + 3), 16));
        j += 2;
      } else {
        bytes.push(encodedText.charCodeAt(j));
      }
    }
    var chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    var out = "";
    for (var i = 0; i < bytes.length; i += 3) {
      var b1 = bytes[i];
      var b2 = i + 1 < bytes.length ? bytes[i + 1] : 0;
      var b3 = i + 2 < bytes.length ? bytes[i + 2] : 0;
      var triplet = (b1 << 16) | (b2 << 8) | b3;
      out += chars[(triplet >> 18) & 63];
      out += chars[(triplet >> 12) & 63];
      out += i + 1 < bytes.length ? chars[(triplet >> 6) & 63] : "=";
      out += i + 2 < bytes.length ? chars[triplet & 63] : "=";
    }
    return out;
  }

  // ---------------------------------------------------------------------------
  // zeb/use — safe server-side fallbacks (the bundle only runs client-side)
  // ---------------------------------------------------------------------------
  libs["zeb/use"] = {
    useDebounce: function (value) { return value; },
    useThrottle: function (value) { return value; },
    useLocalStorage: function (key, init) { return [init, function () {}]; },
    useClipboard: function () { return { copied: false, copy: function () {} }; },
    useTemporaryState: function (init) { return [init, function () {}]; },
    useWindowEvent: function () {},
    useLazyModule: function () { return [null, true, null]; },
    // useSearchParams deliberately absent here: it belongs to zeb/react.
    useSplitPane: function () { return { current: null }; },
    useClickAway: function () { return { current: null }; },
    useInterval: function () {},
    useGeolocation: function () { return { loading: true, error: null, coords: null }; },
    useTree: function () {
      return { expanded: new Set(), isExpanded: function () { return false; }, toggle: function () {}, expand: function () {}, collapse: function () {}, expandAll: function () {}, collapseAll: function () {} };
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/rooms — a room is joined in the browser, from an effect; on the server
  // it is always still connecting, and nothing is sent.
  // ---------------------------------------------------------------------------
  libs["zeb/rooms"] = {
    useRoom: function () {
      return { state: undefined, status: "connecting", send: function () { return false; }, room: null };
    },
    joinRoom: function (name) {
      var off = function () {};
      return {
        name: String(name), state: undefined, status: "connecting", sessionId: null, closed: null,
        on: function () { return off; }, onState: function () { return off; }, onStatus: function () { return off; },
        send: function () { return false; }, leave: function () {}
      };
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/prosemirror — engine only, browser only. `zeb/ui/editor` renders a
  // document on the server itself and mounts the engine in an effect, so
  // nothing here needs a server-side stand-in.
  // zeb/pdf — functions a page calls from the browser.
  // ---------------------------------------------------------------------------
  libs["zeb/prosemirror"] = {};
  libs["zeb/pdf"] = {};

  // ---------------------------------------------------------------------------
  // zeb/threejs — Three.js is WebGL/browser-only.
  // Canvas/ThreeCanvas/ThreeScene render placeholder divs.
  // Three.js classes are empty constructors — only ever called inside useEffect,
  // which does not run during SSR renderToString.
  // ---------------------------------------------------------------------------
  (function () {
    var three = {
      Canvas: function (props) {
        return h('div', { className: (props && props.className) || 'w-full h-full' });
      },
      ThreeContext: createContext(null),
      ThreeScene: function (props) {
        return h('div', {
          'data-zeb-lib': 'threejs',
          'data-zeb-wrapper': 'ThreeScene',
          'data-config': JSON.stringify((props && props.config) || {}),
          id: props && props.id,
          className: (props && props.className) || 'w-full h-full',
        });
      },
      useThree: function () { return {}; },
      useFrame: function () {},
      OrbitControls: function () { return null; },
      createSceneRuntime: function () { return {}; },
      mountThreeScene: function () {},
      ensureThree: function () { return {}; },
      MathUtils: {},
      REVISION: '183',
    };
    three.ThreeCanvas = three.Canvas;
    var _cls = function () {};
    var _names = [
      'Scene', 'PerspectiveCamera', 'OrthographicCamera', 'WebGLRenderer',
      'Mesh', 'Group', 'Object3D', 'InstancedMesh', 'Points', 'Line',
      'BoxGeometry', 'SphereGeometry', 'PlaneGeometry', 'CylinderGeometry',
      'TorusGeometry', 'TorusKnotGeometry', 'ConeGeometry', 'RingGeometry', 'CircleGeometry', 'BufferGeometry',
      'MeshStandardMaterial', 'MeshBasicMaterial', 'MeshPhongMaterial', 'MeshLambertMaterial',
      'MeshNormalMaterial', 'MeshToonMaterial', 'MeshPhysicalMaterial', 'ShaderMaterial',
      'DirectionalLight', 'PointLight', 'SpotLight', 'AmbientLight', 'HemisphereLight',
      'Vector2', 'Vector3', 'Vector4', 'Quaternion', 'Euler', 'Matrix4', 'Color',
      'Raycaster', 'Clock', 'AnimationMixer', 'TextureLoader', 'CubeTextureLoader', 'Texture'
    ];
    for (var i = 0; i < _names.length; i++) { three[_names[i]] = _cls; }
    libs["zeb/threejs"] = three;
  })();

  // ---------------------------------------------------------------------------
  // zeb/threejs-vrm — the VRM viewer is WebGL/browser-only.
  // ---------------------------------------------------------------------------
  libs["zeb/threejs-vrm"] = {
    VrmViewer: function (props) {
      var cfg = JSON.stringify({
        modelUrl: (props && (props.modelUrl || props.model_url)) || '',
        height: (props && props.height) || '400px',
        background: (props && props.background) || 'transparent',
        autoRotate: !!(props && props.autoRotate),
        cameraZ: (props && props.cameraZ) || 1.5,
      });
      return h('div', {
        'data-zeb-lib': 'threejs-vrm',
        'data-zeb-wrapper': 'VrmViewer',
        'data-config': cfg,
        id: props && props.id,
        className: (props && props.className) || 'w-full h-full',
        style: { width: '100%', height: (props && props.height) || '400px' },
      });
    },
    mountVrmViewer: function () {},
  };

  // ---------------------------------------------------------------------------
  // zeb/deckgl — Deck.gl is WebGL/browser-only.
  // ---------------------------------------------------------------------------
  libs["zeb/deckgl"] = {
    DeckMap: function (props) {
      var cfg = JSON.stringify({
        initialViewState: props && props.initialViewState,
        controller: !props || props.controller !== false,
        layers: (props && props.layers) || [],
        stateKey: (props && props.stateKey) || null,
        layerKey: (props && props.layerKey) || null,
        background: (props && props.background) || 'transparent',
      });
      return h('div', {
        'data-zeb-lib': 'deckgl',
        'data-zeb-wrapper': 'DeckMap',
        'data-config': cfg,
        id: props && props.id,
        className: props && props.className,
        style: { width: '100%', height: (props && props.height) || '400px' },
      });
    },
    deckgl: {},
    buildLayer: function () { return null; },
    buildLayers: function () { return []; },
    mountDeckMap: function () {},
    ensureDeck: function () {},
    createDeckMapRuntime: function () { return {}; },
  };

  // ---------------------------------------------------------------------------
  // zeb/d3 — D3 chart components render placeholder divs.
  // ---------------------------------------------------------------------------
  libs["zeb/d3"] = {
    d3: {},
    useD3: function (callback, deps) {
      var ref = useRef(null);
      useEffect(function () {
        if (!ref.current) return;
        return callback(ref.current, {});
      }, deps || []);
      return ref;
    },
    D3Bars: function (props) {
      var cfg = JSON.stringify({
        type: (props && props.type) || 'bar',
        data: (props && props.data) || [],
        xKey: props && props.xKey,
        yKey: props && props.yKey,
        stateKey: props && props.stateKey,
        height: (props && props.height) || '260px',
        colorScheme: props && props.colorScheme,
        area: !!(props && props.area),
      });
      return h('div', {
        'data-zeb-lib': 'd3',
        'data-zeb-wrapper': 'D3Bars',
        'data-config': cfg,
        id: props && props.id,
        className: props && props.className,
        style: { width: '100%', height: (props && props.height) || '260px' },
      });
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/graphui — the graph canvas is browser-only.
  // ---------------------------------------------------------------------------
  libs["zeb/graphui"] = {
    GraphCanvas: function (props) {
      return h('div', {
        'data-zeb-lib': 'graphui',
        'data-zeb-wrapper': 'GraphCanvas',
        id: props && props.id,
        className: (props && props.className) || 'w-full h-full',
      });
    },
    PipelineGraph: function PipelineGraph(props) {
      return h('div', {
        'data-zeb-lib': 'graphui',
        'data-zeb-wrapper': 'PipelineGraph',
        id: props && props.id,
        className: (props && props.className) || 'w-full h-full',
      });
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/codemirror — the code editor is browser-only.
  // ---------------------------------------------------------------------------
  libs["zeb/codemirror"] = {
    CodeEditor: function (props) {
      return h('div', {
        'data-zeb-lib': 'codemirror',
        'data-zeb-wrapper': 'CodeEditor',
        id: props && props.id,
        className: (props && props.className) || 'w-full h-full',
      });
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/markdown — the Markdown component renders an encoded placeholder.
  // ---------------------------------------------------------------------------
  libs["zeb/markdown"] = {
    Markdown: function (props) {
      var text = (props && props.content) || (typeof (props && props.children) === 'string' ? props.children : '') || '';
      var encoded = base64EncodeUtf8(text);
      return h('div', {
        'data-zeb-lib': 'markdown',
        'data-rwe-md': encoded,
        className: 'rwe-md-placeholder' + ((props && props.className) ? (' ' + props.className) : ''),
      });
    },
  };

  // ---------------------------------------------------------------------------
  // zeb/potoru — the player and the compiler are browser-only.
  // Same markup as the runtime's client components (0.1/runtime/entry.mjs) and
  // the wrappers (0.1/wrappers/*.tsx), so hydration matches.
  // ---------------------------------------------------------------------------
  (function () {
    function cssSize(value) { return typeof value === 'number' ? value + 'px' : String(value); }
    function boxStyle(height, aspect) {
      var style = { width: '100%', display: 'block', position: 'relative' };
      if (height) style.height = cssSize(height);
      if (aspect) style.aspectRatio = String(aspect).replace(':', ' / ');
      return style;
    }
    var PotoPlayer = function (props) {
      var p = props || {};
      var libraries = Array.isArray(p.libraries) ? p.libraries : String(p.libraries || '').split(/[\s,]+/).filter(Boolean);
      var config = { src: p.src || '' };
      if (libraries.length) config.libraries = libraries;
      ['controls', 'autoplay', 'muted', 'loop', 'captions', 'still'].forEach(function (k) { if (p[k]) config[k] = true; });
      ['time', 'mode', 'renderer', 'height', 'aspect', 'fit', 'camera', 'voices', 'voiceFallback'].forEach(function (k) { if (p[k] !== undefined && p[k] !== null && p[k] !== '') config[k] = p[k]; });
      return h('div', {
        'data-zeb-lib': 'potoru',
        'data-zeb-wrapper': 'PotoPlayer',
        'data-config': JSON.stringify(config),
        id: p.id,
        className: p.className,
        style: boxStyle(config.height, config.aspect),
      });
    };
    var PotoSnapshot = function (props) {
      var p = props || {};
      var libraries = Array.isArray(p.libraries) ? p.libraries : String(p.libraries || '').split(/[\s,]+/).filter(Boolean);
      var config = { src: p.src || '' };
      ['time', 'camera', 'fit', 'height', 'aspect'].forEach(function (k) { if (p[k] !== undefined && p[k] !== null && p[k] !== '') config[k] = p[k]; });
      if (libraries.length) config.libraries = libraries;
      return h('div', {
        'data-zeb-lib': 'potoru',
        'data-zeb-wrapper': 'PotoSnapshot',
        'data-config': JSON.stringify(config),
        id: p.id,
        className: p.className,
        style: boxStyle(config.height, config.aspect),
      });
    };
    var PotoEditor = function (props) {
      var p = props || {};
      var config = { mode: p.mode === 'script' ? 'script' : 'yaml' };
      if (p.yaml) config.yaml = p.yaml;
      if (p.script) config.script = p.script;
      if (p.height) config.height = p.height;
      return h('div', {
        'data-zeb-lib': 'potoru',
        'data-zeb-wrapper': 'PotoEditor',
        'data-config': JSON.stringify(config),
        id: p.id,
        className: p.className,
        style: boxStyle(config.height || '560px'),
      });
    };
    // usePotoPlayer: on the server there is no player yet; every call does nothing.
    var usePotoPlayer = function (id) {
      var nothing = function () { return undefined; };
      return { id: id, instance: undefined, apply: nothing, set: nothing, fire: nothing, play: nothing, pause: nothing, seek: nothing, restart: nothing, setCamera: nothing, load: nothing };
    };
    var browserOnly = function () { return Promise.reject(new Error('zeb/potoru runs in the browser')); };
    libs["zeb/potoru"] = {
      PotoPlayer: PotoPlayer,
      PotoSnapshot: PotoSnapshot,
      PotoEditor: PotoEditor,
      usePotoPlayer: usePotoPlayer,
      mountPotoPlayer: browserOnly,
      mountPotoSnapshot: browserOnly,
      mountPotoEditor: browserOnly,
      compile: browserOnly,
      loadPlayer: browserOnly,
      potoru: { version: '0.1.3', get: function () { return undefined; }, getEditor: function () { return undefined; }, mountPotoPlayer: browserOnly, mountPotoSnapshot: browserOnly, mountPotoEditor: browserOnly, compile: browserOnly, loadPlayer: browserOnly, usePotoPlayer: usePotoPlayer },
    };
  })();

  // ---------------------------------------------------------------------------
  // zeb/world — aslnut draws on a canvas it makes in the browser. The server
  // renders the box <World> renders before that canvas exists (the same markup,
  // so hydration matches); the hooks answer "no world yet". aslnut has hundreds
  // of other names (earth, imagery, terrain, plane, layer, data, scale, theme,
  // shape, …) a page calls while it renders, to make the options and elements it
  // hands <World>. Each one is one inert stand-in: the library entry answers it
  // for any name it does not define, and it answers itself to a call, a
  // construction and a property read, so `data.geojson(url)` and
  // `scale.category(fn)` run and do nothing. Four keys stay undefined: `then`
  // (an await would wait for ever), Symbol.iterator, Symbol.toPrimitive and
  // `toJSON`.
  // ---------------------------------------------------------------------------
  (function () {
    function undefinedKey(key) {
      return key === 'then' || key === 'toJSON' || key === Symbol.iterator || key === Symbol.toPrimitive;
    }
    var inert = new Proxy(function () {}, {
      get: function (_target, key) { return undefinedKey(key) ? undefined : inert; },
      apply: function () { return inert; },
      construct: function () { return inert; },
    });
    function World(props) {
      var p = props || {};
      return h('div', { className: p.className, style: Object.assign({ position: 'relative' }, p.style) },
        h('div', { style: { position: 'absolute', inset: 0 } }),
        p.children);
    }
    var named = {
      World: World,
      useWorld: function () { return null; },
      useElement: function () { return null; },
      useWorldEvent: function () {},
    };
    libs["zeb/world"] = new Proxy(named, {
      get: function (target, key) {
        if (Object.prototype.hasOwnProperty.call(target, key)) return target[key];
        return undefinedKey(key) ? undefined : inert;
      },
    });
  })();
})();
