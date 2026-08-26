/* Preact 10.28.4 — MIT License © Jason Miller (https://preactjs.com) */
var __defProp = Object.defineProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};

// node_modules/preact/dist/preact.module.js
var n;
var l;
var u;
var t;
var i;
var r;
var o;
var e;
var f;
var c;
var s;
var a;
var h;
var p = {};
var v = [];
var y = /acit|ex(?:s|g|n|p|$)|rph|grid|ows|mnc|ntw|ine[ch]|zoo|^ord|itera/i;
var d = Array.isArray;
function w(n3, l4) {
  for (var u4 in l4) n3[u4] = l4[u4];
  return n3;
}
function g(n3) {
  n3 && n3.parentNode && n3.parentNode.removeChild(n3);
}
function _(l4, u4, t4) {
  var i4, r3, o4, e3 = {};
  for (o4 in u4) "key" == o4 ? i4 = u4[o4] : "ref" == o4 ? r3 = u4[o4] : e3[o4] = u4[o4];
  if (arguments.length > 2 && (e3.children = arguments.length > 3 ? n.call(arguments, 2) : t4), "function" == typeof l4 && null != l4.defaultProps) for (o4 in l4.defaultProps) void 0 === e3[o4] && (e3[o4] = l4.defaultProps[o4]);
  return m(l4, e3, i4, r3, null);
}
function m(n3, t4, i4, r3, o4) {
  var e3 = { type: n3, props: t4, key: i4, ref: r3, __k: null, __: null, __b: 0, __e: null, __c: null, constructor: void 0, __v: null == o4 ? ++u : o4, __i: -1, __u: 0 };
  return null == o4 && null != l.vnode && l.vnode(e3), e3;
}
function b() {
  return { current: null };
}
function k(n3) {
  return n3.children;
}
function x(n3, l4) {
  this.props = n3, this.context = l4;
}
function S(n3, l4) {
  if (null == l4) return n3.__ ? S(n3.__, n3.__i + 1) : null;
  for (var u4; l4 < n3.__k.length; l4++) if (null != (u4 = n3.__k[l4]) && null != u4.__e) return u4.__e;
  return "function" == typeof n3.type ? S(n3) : null;
}
function C(n3) {
  if (n3.__P && n3.__d) {
    var u4 = n3.__v, t4 = u4.__e, i4 = [], r3 = [], o4 = w({}, u4);
    o4.__v = u4.__v + 1, l.vnode && l.vnode(o4), z(n3.__P, o4, u4, n3.__n, n3.__P.namespaceURI, 32 & u4.__u ? [t4] : null, i4, null == t4 ? S(u4) : t4, !!(32 & u4.__u), r3), o4.__v = u4.__v, o4.__.__k[o4.__i] = o4, V(i4, o4, r3), u4.__e = u4.__ = null, o4.__e != t4 && M(o4);
  }
}
function M(n3) {
  if (null != (n3 = n3.__) && null != n3.__c) return n3.__e = n3.__c.base = null, n3.__k.some(function(l4) {
    if (null != l4 && null != l4.__e) return n3.__e = n3.__c.base = l4.__e;
  }), M(n3);
}
function $(n3) {
  (!n3.__d && (n3.__d = true) && i.push(n3) && !I.__r++ || r != l.debounceRendering) && ((r = l.debounceRendering) || o)(I);
}
function I() {
  for (var n3, l4 = 1; i.length; ) i.length > l4 && i.sort(e), n3 = i.shift(), l4 = i.length, C(n3);
  I.__r = 0;
}
function P(n3, l4, u4, t4, i4, r3, o4, e3, f4, c4, s4) {
  var a4, h3, y3, d3, w3, g4, _3, m3 = t4 && t4.__k || v, b3 = l4.length;
  for (f4 = A(u4, l4, m3, f4, b3), a4 = 0; a4 < b3; a4++) null != (y3 = u4.__k[a4]) && (h3 = -1 != y3.__i && m3[y3.__i] || p, y3.__i = a4, g4 = z(n3, y3, h3, i4, r3, o4, e3, f4, c4, s4), d3 = y3.__e, y3.ref && h3.ref != y3.ref && (h3.ref && D(h3.ref, null, y3), s4.push(y3.ref, y3.__c || d3, y3)), null == w3 && null != d3 && (w3 = d3), (_3 = !!(4 & y3.__u)) || h3.__k === y3.__k ? f4 = H(y3, f4, n3, _3) : "function" == typeof y3.type && void 0 !== g4 ? f4 = g4 : d3 && (f4 = d3.nextSibling), y3.__u &= -7);
  return u4.__e = w3, f4;
}
function A(n3, l4, u4, t4, i4) {
  var r3, o4, e3, f4, c4, s4 = u4.length, a4 = s4, h3 = 0;
  for (n3.__k = new Array(i4), r3 = 0; r3 < i4; r3++) null != (o4 = l4[r3]) && "boolean" != typeof o4 && "function" != typeof o4 ? ("string" == typeof o4 || "number" == typeof o4 || "bigint" == typeof o4 || o4.constructor == String ? o4 = n3.__k[r3] = m(null, o4, null, null, null) : d(o4) ? o4 = n3.__k[r3] = m(k, { children: o4 }, null, null, null) : void 0 === o4.constructor && o4.__b > 0 ? o4 = n3.__k[r3] = m(o4.type, o4.props, o4.key, o4.ref ? o4.ref : null, o4.__v) : n3.__k[r3] = o4, f4 = r3 + h3, o4.__ = n3, o4.__b = n3.__b + 1, e3 = null, -1 != (c4 = o4.__i = T(o4, u4, f4, a4)) && (a4--, (e3 = u4[c4]) && (e3.__u |= 2)), null == e3 || null == e3.__v ? (-1 == c4 && (i4 > s4 ? h3-- : i4 < s4 && h3++), "function" != typeof o4.type && (o4.__u |= 4)) : c4 != f4 && (c4 == f4 - 1 ? h3-- : c4 == f4 + 1 ? h3++ : (c4 > f4 ? h3-- : h3++, o4.__u |= 4))) : n3.__k[r3] = null;
  if (a4) for (r3 = 0; r3 < s4; r3++) null != (e3 = u4[r3]) && 0 == (2 & e3.__u) && (e3.__e == t4 && (t4 = S(e3)), E(e3, e3));
  return t4;
}
function H(n3, l4, u4, t4) {
  var i4, r3;
  if ("function" == typeof n3.type) {
    for (i4 = n3.__k, r3 = 0; i4 && r3 < i4.length; r3++) i4[r3] && (i4[r3].__ = n3, l4 = H(i4[r3], l4, u4, t4));
    return l4;
  }
  n3.__e != l4 && (t4 && (l4 && n3.type && !l4.parentNode && (l4 = S(n3)), u4.insertBefore(n3.__e, l4 || null)), l4 = n3.__e);
  do {
    l4 = l4 && l4.nextSibling;
  } while (null != l4 && 8 == l4.nodeType);
  return l4;
}
function L(n3, l4) {
  return l4 = l4 || [], null == n3 || "boolean" == typeof n3 || (d(n3) ? n3.some(function(n4) {
    L(n4, l4);
  }) : l4.push(n3)), l4;
}
function T(n3, l4, u4, t4) {
  var i4, r3, o4, e3 = n3.key, f4 = n3.type, c4 = l4[u4], s4 = null != c4 && 0 == (2 & c4.__u);
  if (null === c4 && null == e3 || s4 && e3 == c4.key && f4 == c4.type) return u4;
  if (t4 > (s4 ? 1 : 0)) {
    for (i4 = u4 - 1, r3 = u4 + 1; i4 >= 0 || r3 < l4.length; ) if (null != (c4 = l4[o4 = i4 >= 0 ? i4-- : r3++]) && 0 == (2 & c4.__u) && e3 == c4.key && f4 == c4.type) return o4;
  }
  return -1;
}
function j(n3, l4, u4) {
  "-" == l4[0] ? n3.setProperty(l4, null == u4 ? "" : u4) : n3[l4] = null == u4 ? "" : "number" != typeof u4 || y.test(l4) ? u4 : u4 + "px";
}
function F(n3, l4, u4, t4, i4) {
  var r3, o4;
  n: if ("style" == l4) if ("string" == typeof u4) n3.style.cssText = u4;
  else {
    if ("string" == typeof t4 && (n3.style.cssText = t4 = ""), t4) for (l4 in t4) u4 && l4 in u4 || j(n3.style, l4, "");
    if (u4) for (l4 in u4) t4 && u4[l4] == t4[l4] || j(n3.style, l4, u4[l4]);
  }
  else if ("o" == l4[0] && "n" == l4[1]) r3 = l4 != (l4 = l4.replace(f, "$1")), o4 = l4.toLowerCase(), l4 = o4 in n3 || "onFocusOut" == l4 || "onFocusIn" == l4 ? o4.slice(2) : l4.slice(2), n3.l || (n3.l = {}), n3.l[l4 + r3] = u4, u4 ? t4 ? u4.u = t4.u : (u4.u = c, n3.addEventListener(l4, r3 ? a : s, r3)) : n3.removeEventListener(l4, r3 ? a : s, r3);
  else {
    if ("http://www.w3.org/2000/svg" == i4) l4 = l4.replace(/xlink(H|:h)/, "h").replace(/sName$/, "s");
    else if ("width" != l4 && "height" != l4 && "href" != l4 && "list" != l4 && "form" != l4 && "tabIndex" != l4 && "download" != l4 && "rowSpan" != l4 && "colSpan" != l4 && "role" != l4 && "popover" != l4 && l4 in n3) try {
      n3[l4] = null == u4 ? "" : u4;
      break n;
    } catch (n4) {
    }
    "function" == typeof u4 || (null == u4 || false === u4 && "-" != l4[4] ? n3.removeAttribute(l4) : n3.setAttribute(l4, "popover" == l4 && 1 == u4 ? "" : u4));
  }
}
function O(n3) {
  return function(u4) {
    if (this.l) {
      var t4 = this.l[u4.type + n3];
      if (null == u4.t) u4.t = c++;
      else if (u4.t < t4.u) return;
      return t4(l.event ? l.event(u4) : u4);
    }
  };
}
function z(n3, u4, t4, i4, r3, o4, e3, f4, c4, s4) {
  var a4, h3, p4, y3, _3, m3, b3, S2, C3, M3, $3, I2, A4, H3, L2, T4 = u4.type;
  if (void 0 !== u4.constructor) return null;
  128 & t4.__u && (c4 = !!(32 & t4.__u), o4 = [f4 = u4.__e = t4.__e]), (a4 = l.__b) && a4(u4);
  n: if ("function" == typeof T4) try {
    if (S2 = u4.props, C3 = "prototype" in T4 && T4.prototype.render, M3 = (a4 = T4.contextType) && i4[a4.__c], $3 = a4 ? M3 ? M3.props.value : a4.__ : i4, t4.__c ? b3 = (h3 = u4.__c = t4.__c).__ = h3.__E : (C3 ? u4.__c = h3 = new T4(S2, $3) : (u4.__c = h3 = new x(S2, $3), h3.constructor = T4, h3.render = G), M3 && M3.sub(h3), h3.state || (h3.state = {}), h3.__n = i4, p4 = h3.__d = true, h3.__h = [], h3._sb = []), C3 && null == h3.__s && (h3.__s = h3.state), C3 && null != T4.getDerivedStateFromProps && (h3.__s == h3.state && (h3.__s = w({}, h3.__s)), w(h3.__s, T4.getDerivedStateFromProps(S2, h3.__s))), y3 = h3.props, _3 = h3.state, h3.__v = u4, p4) C3 && null == T4.getDerivedStateFromProps && null != h3.componentWillMount && h3.componentWillMount(), C3 && null != h3.componentDidMount && h3.__h.push(h3.componentDidMount);
    else {
      if (C3 && null == T4.getDerivedStateFromProps && S2 !== y3 && null != h3.componentWillReceiveProps && h3.componentWillReceiveProps(S2, $3), u4.__v == t4.__v || !h3.__e && null != h3.shouldComponentUpdate && false === h3.shouldComponentUpdate(S2, h3.__s, $3)) {
        u4.__v != t4.__v && (h3.props = S2, h3.state = h3.__s, h3.__d = false), u4.__e = t4.__e, u4.__k = t4.__k, u4.__k.some(function(n4) {
          n4 && (n4.__ = u4);
        }), v.push.apply(h3.__h, h3._sb), h3._sb = [], h3.__h.length && e3.push(h3);
        break n;
      }
      null != h3.componentWillUpdate && h3.componentWillUpdate(S2, h3.__s, $3), C3 && null != h3.componentDidUpdate && h3.__h.push(function() {
        h3.componentDidUpdate(y3, _3, m3);
      });
    }
    if (h3.context = $3, h3.props = S2, h3.__P = n3, h3.__e = false, I2 = l.__r, A4 = 0, C3) h3.state = h3.__s, h3.__d = false, I2 && I2(u4), a4 = h3.render(h3.props, h3.state, h3.context), v.push.apply(h3.__h, h3._sb), h3._sb = [];
    else do {
      h3.__d = false, I2 && I2(u4), a4 = h3.render(h3.props, h3.state, h3.context), h3.state = h3.__s;
    } while (h3.__d && ++A4 < 25);
    h3.state = h3.__s, null != h3.getChildContext && (i4 = w(w({}, i4), h3.getChildContext())), C3 && !p4 && null != h3.getSnapshotBeforeUpdate && (m3 = h3.getSnapshotBeforeUpdate(y3, _3)), H3 = null != a4 && a4.type === k && null == a4.key ? q(a4.props.children) : a4, f4 = P(n3, d(H3) ? H3 : [H3], u4, t4, i4, r3, o4, e3, f4, c4, s4), h3.base = u4.__e, u4.__u &= -161, h3.__h.length && e3.push(h3), b3 && (h3.__E = h3.__ = null);
  } catch (n4) {
    if (u4.__v = null, c4 || null != o4) if (n4.then) {
      for (u4.__u |= c4 ? 160 : 128; f4 && 8 == f4.nodeType && f4.nextSibling; ) f4 = f4.nextSibling;
      o4[o4.indexOf(f4)] = null, u4.__e = f4;
    } else {
      for (L2 = o4.length; L2--; ) g(o4[L2]);
      N(u4);
    }
    else u4.__e = t4.__e, u4.__k = t4.__k, n4.then || N(u4);
    l.__e(n4, u4, t4);
  }
  else null == o4 && u4.__v == t4.__v ? (u4.__k = t4.__k, u4.__e = t4.__e) : f4 = u4.__e = B(t4.__e, u4, t4, i4, r3, o4, e3, c4, s4);
  return (a4 = l.diffed) && a4(u4), 128 & u4.__u ? void 0 : f4;
}
function N(n3) {
  n3 && (n3.__c && (n3.__c.__e = true), n3.__k && n3.__k.some(N));
}
function V(n3, u4, t4) {
  for (var i4 = 0; i4 < t4.length; i4++) D(t4[i4], t4[++i4], t4[++i4]);
  l.__c && l.__c(u4, n3), n3.some(function(u5) {
    try {
      n3 = u5.__h, u5.__h = [], n3.some(function(n4) {
        n4.call(u5);
      });
    } catch (n4) {
      l.__e(n4, u5.__v);
    }
  });
}
function q(n3) {
  return "object" != typeof n3 || null == n3 || n3.__b > 0 ? n3 : d(n3) ? n3.map(q) : w({}, n3);
}
function B(u4, t4, i4, r3, o4, e3, f4, c4, s4) {
  var a4, h3, v3, y3, w3, _3, m3, b3 = i4.props || p, k3 = t4.props, x4 = t4.type;
  if ("svg" == x4 ? o4 = "http://www.w3.org/2000/svg" : "math" == x4 ? o4 = "http://www.w3.org/1998/Math/MathML" : o4 || (o4 = "http://www.w3.org/1999/xhtml"), null != e3) {
    for (a4 = 0; a4 < e3.length; a4++) if ((w3 = e3[a4]) && "setAttribute" in w3 == !!x4 && (x4 ? w3.localName == x4 : 3 == w3.nodeType)) {
      u4 = w3, e3[a4] = null;
      break;
    }
  }
  if (null == u4) {
    if (null == x4) return document.createTextNode(k3);
    u4 = document.createElementNS(o4, x4, k3.is && k3), c4 && (l.__m && l.__m(t4, e3), c4 = false), e3 = null;
  }
  if (null == x4) b3 === k3 || c4 && u4.data == k3 || (u4.data = k3);
  else {
    if (e3 = e3 && n.call(u4.childNodes), !c4 && null != e3) for (b3 = {}, a4 = 0; a4 < u4.attributes.length; a4++) b3[(w3 = u4.attributes[a4]).name] = w3.value;
    for (a4 in b3) w3 = b3[a4], "dangerouslySetInnerHTML" == a4 ? v3 = w3 : "children" == a4 || a4 in k3 || "value" == a4 && "defaultValue" in k3 || "checked" == a4 && "defaultChecked" in k3 || F(u4, a4, null, w3, o4);
    for (a4 in k3) w3 = k3[a4], "children" == a4 ? y3 = w3 : "dangerouslySetInnerHTML" == a4 ? h3 = w3 : "value" == a4 ? _3 = w3 : "checked" == a4 ? m3 = w3 : c4 && "function" != typeof w3 || b3[a4] === w3 || F(u4, a4, w3, b3[a4], o4);
    if (h3) c4 || v3 && (h3.__html == v3.__html || h3.__html == u4.innerHTML) || (u4.innerHTML = h3.__html), t4.__k = [];
    else if (v3 && (u4.innerHTML = ""), P("template" == t4.type ? u4.content : u4, d(y3) ? y3 : [y3], t4, i4, r3, "foreignObject" == x4 ? "http://www.w3.org/1999/xhtml" : o4, e3, f4, e3 ? e3[0] : i4.__k && S(i4, 0), c4, s4), null != e3) for (a4 = e3.length; a4--; ) g(e3[a4]);
    c4 || (a4 = "value", "progress" == x4 && null == _3 ? u4.removeAttribute("value") : null != _3 && (_3 !== u4[a4] || "progress" == x4 && !_3 || "option" == x4 && _3 != b3[a4]) && F(u4, a4, _3, b3[a4], o4), a4 = "checked", null != m3 && m3 != u4[a4] && F(u4, a4, m3, b3[a4], o4));
  }
  return u4;
}
function D(n3, u4, t4) {
  try {
    if ("function" == typeof n3) {
      var i4 = "function" == typeof n3.__u;
      i4 && n3.__u(), i4 && null == u4 || (n3.__u = n3(u4));
    } else n3.current = u4;
  } catch (n4) {
    l.__e(n4, t4);
  }
}
function E(n3, u4, t4) {
  var i4, r3;
  if (l.unmount && l.unmount(n3), (i4 = n3.ref) && (i4.current && i4.current != n3.__e || D(i4, null, u4)), null != (i4 = n3.__c)) {
    if (i4.componentWillUnmount) try {
      i4.componentWillUnmount();
    } catch (n4) {
      l.__e(n4, u4);
    }
    i4.base = i4.__P = null;
  }
  if (i4 = n3.__k) for (r3 = 0; r3 < i4.length; r3++) i4[r3] && E(i4[r3], u4, t4 || "function" != typeof n3.type);
  t4 || g(n3.__e), n3.__c = n3.__ = n3.__e = void 0;
}
function G(n3, l4, u4) {
  return this.constructor(n3, u4);
}
function J(u4, t4, i4) {
  var r3, o4, e3, f4;
  t4 == document && (t4 = document.documentElement), l.__ && l.__(u4, t4), o4 = (r3 = "function" == typeof i4) ? null : i4 && i4.__k || t4.__k, e3 = [], f4 = [], z(t4, u4 = (!r3 && i4 || t4).__k = _(k, null, [u4]), o4 || p, p, t4.namespaceURI, !r3 && i4 ? [i4] : o4 ? null : t4.firstChild ? n.call(t4.childNodes) : null, e3, !r3 && i4 ? i4 : o4 ? o4.__e : t4.firstChild, r3, f4), V(e3, u4, f4);
}
function K(n3, l4) {
  J(n3, l4, K);
}
function Q(l4, u4, t4) {
  var i4, r3, o4, e3, f4 = w({}, l4.props);
  for (o4 in l4.type && l4.type.defaultProps && (e3 = l4.type.defaultProps), u4) "key" == o4 ? i4 = u4[o4] : "ref" == o4 ? r3 = u4[o4] : f4[o4] = void 0 === u4[o4] && null != e3 ? e3[o4] : u4[o4];
  return arguments.length > 2 && (f4.children = arguments.length > 3 ? n.call(arguments, 2) : t4), m(l4.type, f4, i4 || l4.key, r3 || l4.ref, null);
}
function R(n3) {
  function l4(n4) {
    var u4, t4;
    return this.getChildContext || (u4 = /* @__PURE__ */ new Set(), (t4 = {})[l4.__c] = this, this.getChildContext = function() {
      return t4;
    }, this.componentWillUnmount = function() {
      u4 = null;
    }, this.shouldComponentUpdate = function(n5) {
      this.props.value != n5.value && u4.forEach(function(n6) {
        n6.__e = true, $(n6);
      });
    }, this.sub = function(n5) {
      u4.add(n5);
      var l5 = n5.componentWillUnmount;
      n5.componentWillUnmount = function() {
        u4 && u4.delete(n5), l5 && l5.call(n5);
      };
    }), n4.children;
  }
  return l4.__c = "__cC" + h++, l4.__ = n3, l4.Provider = l4.__l = (l4.Consumer = function(n4, l5) {
    return n4.children(l5);
  }).contextType = l4, l4;
}
n = v.slice, l = { __e: function(n3, l4, u4, t4) {
  for (var i4, r3, o4; l4 = l4.__; ) if ((i4 = l4.__c) && !i4.__) try {
    if ((r3 = i4.constructor) && null != r3.getDerivedStateFromError && (i4.setState(r3.getDerivedStateFromError(n3)), o4 = i4.__d), null != i4.componentDidCatch && (i4.componentDidCatch(n3, t4 || {}), o4 = i4.__d), o4) return i4.__E = i4;
  } catch (l5) {
    n3 = l5;
  }
  throw n3;
} }, u = 0, t = function(n3) {
  return null != n3 && void 0 === n3.constructor;
}, x.prototype.setState = function(n3, l4) {
  var u4;
  u4 = null != this.__s && this.__s != this.state ? this.__s : this.__s = w({}, this.state), "function" == typeof n3 && (n3 = n3(w({}, u4), this.props)), n3 && w(u4, n3), null != n3 && this.__v && (l4 && this._sb.push(l4), $(this));
}, x.prototype.forceUpdate = function(n3) {
  this.__v && (this.__e = true, n3 && this.__h.push(n3), $(this));
}, x.prototype.render = k, i = [], o = "function" == typeof Promise ? Promise.prototype.then.bind(Promise.resolve()) : setTimeout, e = function(n3, l4) {
  return n3.__v.__b - l4.__v.__b;
}, I.__r = 0, f = /(PointerCapture)$|Capture$/i, c = 0, s = O(false), a = O(true), h = 0;

// node_modules/preact/hooks/dist/hooks.module.js
var t2;
var r2;
var u2;
var i2;
var o2 = 0;
var f2 = [];
var c2 = l;
var e2 = c2.__b;
var a2 = c2.__r;
var v2 = c2.diffed;
var l2 = c2.__c;
var m2 = c2.unmount;
var s2 = c2.__;
function p2(n3, t4) {
  c2.__h && c2.__h(r2, n3, o2 || t4), o2 = 0;
  var u4 = r2.__H || (r2.__H = { __: [], __h: [] });
  return n3 >= u4.__.length && u4.__.push({}), u4.__[n3];
}
function d2(n3) {
  return o2 = 1, h2(D2, n3);
}
function h2(n3, u4, i4) {
  var o4 = p2(t2++, 2);
  if (o4.t = n3, !o4.__c && (o4.__ = [i4 ? i4(u4) : D2(void 0, u4), function(n4) {
    var t4 = o4.__N ? o4.__N[0] : o4.__[0], r3 = o4.t(t4, n4);
    t4 !== r3 && (o4.__N = [r3, o4.__[1]], o4.__c.setState({}));
  }], o4.__c = r2, !r2.__f)) {
    var f4 = function(n4, t4, r3) {
      if (!o4.__c.__H) return true;
      var u5 = o4.__c.__H.__.filter(function(n5) {
        return n5.__c;
      });
      if (u5.every(function(n5) {
        return !n5.__N;
      })) return !c4 || c4.call(this, n4, t4, r3);
      var i5 = o4.__c.props !== n4;
      return u5.some(function(n5) {
        if (n5.__N) {
          var t5 = n5.__[0];
          n5.__ = n5.__N, n5.__N = void 0, t5 !== n5.__[0] && (i5 = true);
        }
      }), c4 && c4.call(this, n4, t4, r3) || i5;
    };
    r2.__f = true;
    var c4 = r2.shouldComponentUpdate, e3 = r2.componentWillUpdate;
    r2.componentWillUpdate = function(n4, t4, r3) {
      if (this.__e) {
        var u5 = c4;
        c4 = void 0, f4(n4, t4, r3), c4 = u5;
      }
      e3 && e3.call(this, n4, t4, r3);
    }, r2.shouldComponentUpdate = f4;
  }
  return o4.__N || o4.__;
}
function y2(n3, u4) {
  var i4 = p2(t2++, 3);
  !c2.__s && C2(i4.__H, u4) && (i4.__ = n3, i4.u = u4, r2.__H.__h.push(i4));
}
function _2(n3, u4) {
  var i4 = p2(t2++, 4);
  !c2.__s && C2(i4.__H, u4) && (i4.__ = n3, i4.u = u4, r2.__h.push(i4));
}
function A2(n3) {
  return o2 = 5, T2(function() {
    return { current: n3 };
  }, []);
}
function F2(n3, t4, r3) {
  o2 = 6, _2(function() {
    if ("function" == typeof n3) {
      var r4 = n3(t4());
      return function() {
        n3(null), r4 && "function" == typeof r4 && r4();
      };
    }
    if (n3) return n3.current = t4(), function() {
      return n3.current = null;
    };
  }, null == r3 ? r3 : r3.concat(n3));
}
function T2(n3, r3) {
  var u4 = p2(t2++, 7);
  return C2(u4.__H, r3) && (u4.__ = n3(), u4.__H = r3, u4.__h = n3), u4.__;
}
function q2(n3, t4) {
  return o2 = 8, T2(function() {
    return n3;
  }, t4);
}
function x2(n3) {
  var u4 = r2.context[n3.__c], i4 = p2(t2++, 9);
  return i4.c = n3, u4 ? (null == i4.__ && (i4.__ = true, u4.sub(r2)), u4.props.value) : n3.__;
}
function P2(n3, t4) {
  c2.useDebugValue && c2.useDebugValue(t4 ? t4(n3) : n3);
}
function b2(n3) {
  var u4 = p2(t2++, 10), i4 = d2();
  return u4.__ = n3, r2.componentDidCatch || (r2.componentDidCatch = function(n4, t4) {
    u4.__ && u4.__(n4, t4), i4[1](n4);
  }), [i4[0], function() {
    i4[1](void 0);
  }];
}
function g2() {
  var n3 = p2(t2++, 11);
  if (!n3.__) {
    for (var u4 = r2.__v; null !== u4 && !u4.__m && null !== u4.__; ) u4 = u4.__;
    var i4 = u4.__m || (u4.__m = [0, 0]);
    n3.__ = "P" + i4[0] + "-" + i4[1]++;
  }
  return n3.__;
}
function j2() {
  for (var n3; n3 = f2.shift(); ) {
    var t4 = n3.__H;
    if (n3.__P && t4) try {
      t4.__h.some(z2), t4.__h.some(B2), t4.__h = [];
    } catch (r3) {
      t4.__h = [], c2.__e(r3, n3.__v);
    }
  }
}
c2.__b = function(n3) {
  r2 = null, e2 && e2(n3);
}, c2.__ = function(n3, t4) {
  n3 && t4.__k && t4.__k.__m && (n3.__m = t4.__k.__m), s2 && s2(n3, t4);
}, c2.__r = function(n3) {
  a2 && a2(n3), t2 = 0;
  var i4 = (r2 = n3.__c).__H;
  i4 && (u2 === r2 ? (i4.__h = [], r2.__h = [], i4.__.some(function(n4) {
    n4.__N && (n4.__ = n4.__N), n4.u = n4.__N = void 0;
  })) : (i4.__h.some(z2), i4.__h.some(B2), i4.__h = [], t2 = 0)), u2 = r2;
}, c2.diffed = function(n3) {
  v2 && v2(n3);
  var t4 = n3.__c;
  t4 && t4.__H && (t4.__H.__h.length && (1 !== f2.push(t4) && i2 === c2.requestAnimationFrame || ((i2 = c2.requestAnimationFrame) || w2)(j2)), t4.__H.__.some(function(n4) {
    n4.u && (n4.__H = n4.u), n4.u = void 0;
  })), u2 = r2 = null;
}, c2.__c = function(n3, t4) {
  t4.some(function(n4) {
    try {
      n4.__h.some(z2), n4.__h = n4.__h.filter(function(n5) {
        return !n5.__ || B2(n5);
      });
    } catch (r3) {
      t4.some(function(n5) {
        n5.__h && (n5.__h = []);
      }), t4 = [], c2.__e(r3, n4.__v);
    }
  }), l2 && l2(n3, t4);
}, c2.unmount = function(n3) {
  m2 && m2(n3);
  var t4, r3 = n3.__c;
  r3 && r3.__H && (r3.__H.__.some(function(n4) {
    try {
      z2(n4);
    } catch (n5) {
      t4 = n5;
    }
  }), r3.__H = void 0, t4 && c2.__e(t4, r3.__v));
};
var k2 = "function" == typeof requestAnimationFrame;
function w2(n3) {
  var t4, r3 = function() {
    clearTimeout(u4), k2 && cancelAnimationFrame(t4), setTimeout(n3);
  }, u4 = setTimeout(r3, 35);
  k2 && (t4 = requestAnimationFrame(r3));
}
function z2(n3) {
  var t4 = r2, u4 = n3.__c;
  "function" == typeof u4 && (n3.__c = void 0, u4()), r2 = t4;
}
function B2(n3) {
  var t4 = r2;
  n3.__c = n3.__(), r2 = t4;
}
function C2(n3, t4) {
  return !n3 || n3.length !== t4.length || t4.some(function(t5, r3) {
    return t5 !== n3[r3];
  });
}
function D2(n3, t4) {
  return "function" == typeof t4 ? t4(n3) : t4;
}

// node_modules/preact/compat/dist/compat.module.js
function g3(n3, t4) {
  for (var e3 in t4) n3[e3] = t4[e3];
  return n3;
}
function E2(n3, t4) {
  for (var e3 in n3) if ("__source" !== e3 && !(e3 in t4)) return true;
  for (var r3 in t4) if ("__source" !== r3 && n3[r3] !== t4[r3]) return true;
  return false;
}
function x3(n3) {
  n3();
}
function N2(n3, t4) {
  this.props = n3, this.context = t4;
}
function M2(n3, e3) {
  function r3(n4) {
    var t4 = this.props.ref, r4 = t4 == n4.ref;
    return !r4 && t4 && (t4.call ? t4(null) : t4.current = null), e3 ? !e3(this.props, n4) || !r4 : E2(this.props, n4);
  }
  function u4(e4) {
    return this.shouldComponentUpdate = r3, _(n3, e4);
  }
  return u4.displayName = "Memo(" + (n3.displayName || n3.name) + ")", u4.prototype.isReactComponent = true, u4.__f = true, u4.type = n3, u4;
}
(N2.prototype = new x()).isPureReactComponent = true, N2.prototype.shouldComponentUpdate = function(n3, t4) {
  return E2(this.props, n3) || E2(this.state, t4);
};
var T3 = l.__b;
l.__b = function(n3) {
  n3.type && n3.type.__f && n3.ref && (n3.props.ref = n3.ref, n3.ref = null), T3 && T3(n3);
};
var A3 = "undefined" != typeof Symbol && Symbol.for && /* @__PURE__ */ Symbol.for("react.forward_ref") || 3911;
function D3(n3) {
  function t4(t5) {
    var e3 = g3({}, t5);
    return delete e3.ref, n3(e3, t5.ref || null);
  }
  return t4.$$typeof = A3, t4.render = n3, t4.prototype.isReactComponent = t4.__f = true, t4.displayName = "ForwardRef(" + (n3.displayName || n3.name) + ")", t4;
}
var U = l.__e;
l.__e = function(n3, t4, e3, r3) {
  if (n3.then) {
    for (var u4, o4 = t4; o4 = o4.__; ) if ((u4 = o4.__c) && u4.__c) return null == t4.__e && (t4.__e = e3.__e, t4.__k = e3.__k), u4.__c(n3, t4);
  }
  U(n3, t4, e3, r3);
};
var F3 = l.unmount;
function V2(n3, t4, e3) {
  return n3 && (n3.__c && n3.__c.__H && (n3.__c.__H.__.forEach(function(n4) {
    "function" == typeof n4.__c && n4.__c();
  }), n3.__c.__H = null), null != (n3 = g3({}, n3)).__c && (n3.__c.__P === e3 && (n3.__c.__P = t4), n3.__c.__e = true, n3.__c = null), n3.__k = n3.__k && n3.__k.map(function(n4) {
    return V2(n4, t4, e3);
  })), n3;
}
function W(n3, t4, e3) {
  return n3 && e3 && (n3.__v = null, n3.__k = n3.__k && n3.__k.map(function(n4) {
    return W(n4, t4, e3);
  }), n3.__c && n3.__c.__P === t4 && (n3.__e && e3.appendChild(n3.__e), n3.__c.__e = true, n3.__c.__P = e3)), n3;
}
function P3() {
  this.__u = 0, this.o = null, this.__b = null;
}
function j3(n3) {
  if (!n3.__) return null;
  var t4 = n3.__.__c;
  return t4 && t4.__a && t4.__a(n3);
}
function z3(n3) {
  var e3, r3, u4, o4 = null;
  function i4(i5) {
    if (e3 || (e3 = n3()).then(function(n4) {
      n4 && (o4 = n4.default || n4), u4 = true;
    }, function(n4) {
      r3 = n4, u4 = true;
    }), r3) throw r3;
    if (!u4) throw e3;
    return o4 ? _(o4, i5) : null;
  }
  return i4.displayName = "Lazy", i4.__f = true, i4;
}
function B3() {
  this.i = null, this.l = null;
}
l.unmount = function(n3) {
  var t4 = n3.__c;
  t4 && (t4.__z = true), t4 && t4.__R && t4.__R(), t4 && 32 & n3.__u && (n3.type = null), F3 && F3(n3);
}, (P3.prototype = new x()).__c = function(n3, t4) {
  var e3 = t4.__c, r3 = this;
  null == r3.o && (r3.o = []), r3.o.push(e3);
  var u4 = j3(r3.__v), o4 = false, i4 = function() {
    o4 || r3.__z || (o4 = true, e3.__R = null, u4 ? u4(c4) : c4());
  };
  e3.__R = i4;
  var l4 = e3.__P;
  e3.__P = null;
  var c4 = function() {
    if (!--r3.__u) {
      if (r3.state.__a) {
        var n4 = r3.state.__a;
        r3.__v.__k[0] = W(n4, n4.__c.__P, n4.__c.__O);
      }
      var t5;
      for (r3.setState({ __a: r3.__b = null }); t5 = r3.o.pop(); ) t5.__P = l4, t5.forceUpdate();
    }
  };
  r3.__u++ || 32 & t4.__u || r3.setState({ __a: r3.__b = r3.__v.__k[0] }), n3.then(i4, i4);
}, P3.prototype.componentWillUnmount = function() {
  this.o = [];
}, P3.prototype.render = function(n3, e3) {
  if (this.__b) {
    if (this.__v.__k) {
      var r3 = document.createElement("div"), o4 = this.__v.__k[0].__c;
      this.__v.__k[0] = V2(this.__b, r3, o4.__O = o4.__P);
    }
    this.__b = null;
  }
  var i4 = e3.__a && _(k, null, n3.fallback);
  return i4 && (i4.__u &= -33), [_(k, null, e3.__a ? null : n3.children), i4];
};
var H2 = function(n3, t4, e3) {
  if (++e3[1] === e3[0] && n3.l.delete(t4), n3.props.revealOrder && ("t" !== n3.props.revealOrder[0] || !n3.l.size)) for (e3 = n3.i; e3; ) {
    for (; e3.length > 3; ) e3.pop()();
    if (e3[1] < e3[0]) break;
    n3.i = e3 = e3[2];
  }
};
function Z(n3) {
  return this.getChildContext = function() {
    return n3.context;
  }, n3.children;
}
function Y(n3) {
  var e3 = this, r3 = n3.h;
  if (e3.componentWillUnmount = function() {
    J(null, e3.v), e3.v = null, e3.h = null;
  }, e3.h && e3.h !== r3 && e3.componentWillUnmount(), !e3.v) {
    for (var u4 = e3.__v; null !== u4 && !u4.__m && null !== u4.__; ) u4 = u4.__;
    e3.h = r3, e3.v = { nodeType: 1, parentNode: r3, childNodes: [], __k: { __m: u4.__m }, contains: function() {
      return true;
    }, namespaceURI: r3.namespaceURI, insertBefore: function(n4, t4) {
      this.childNodes.push(n4), e3.h.insertBefore(n4, t4);
    }, removeChild: function(n4) {
      this.childNodes.splice(this.childNodes.indexOf(n4) >>> 1, 1), e3.h.removeChild(n4);
    } };
  }
  J(_(Z, { context: e3.context }, n3.__v), e3.v);
}
function $2(n3, e3) {
  var r3 = _(Y, { __v: n3, h: e3 });
  return r3.containerInfo = e3, r3;
}
(B3.prototype = new x()).__a = function(n3) {
  var t4 = this, e3 = j3(t4.__v), r3 = t4.l.get(n3);
  return r3[0]++, function(u4) {
    var o4 = function() {
      t4.props.revealOrder ? (r3.push(u4), H2(t4, n3, r3)) : u4();
    };
    e3 ? e3(o4) : o4();
  };
}, B3.prototype.render = function(n3) {
  this.i = null, this.l = /* @__PURE__ */ new Map();
  var t4 = L(n3.children);
  n3.revealOrder && "b" === n3.revealOrder[0] && t4.reverse();
  for (var e3 = t4.length; e3--; ) this.l.set(t4[e3], this.i = [1, 0, this.i]);
  return n3.children;
}, B3.prototype.componentDidUpdate = B3.prototype.componentDidMount = function() {
  var n3 = this;
  this.l.forEach(function(t4, e3) {
    H2(n3, e3, t4);
  });
};
var q3 = "undefined" != typeof Symbol && Symbol.for && /* @__PURE__ */ Symbol.for("react.element") || 60103;
var G2 = /^(?:accent|alignment|arabic|baseline|cap|clip(?!PathU)|color|dominant|fill|flood|font|glyph(?!R)|horiz|image(!S)|letter|lighting|marker(?!H|W|U)|overline|paint|pointer|shape|stop|strikethrough|stroke|text(?!L)|transform|underline|unicode|units|v|vector|vert|word|writing|x(?!C))[A-Z]/;
var J2 = /^on(Ani|Tra|Tou|BeforeInp|Compo)/;
var K2 = /[A-Z0-9]/g;
var Q2 = "undefined" != typeof document;
var X = function(n3) {
  return ("undefined" != typeof Symbol && "symbol" == typeof /* @__PURE__ */ Symbol() ? /fil|che|rad/ : /fil|che|ra/).test(n3);
};
x.prototype.isReactComponent = {}, ["componentWillMount", "componentWillReceiveProps", "componentWillUpdate"].forEach(function(t4) {
  Object.defineProperty(x.prototype, t4, { configurable: true, get: function() {
    return this["UNSAFE_" + t4];
  }, set: function(n3) {
    Object.defineProperty(this, t4, { configurable: true, writable: true, value: n3 });
  } });
});
var en = l.event;
function rn() {
}
function un() {
  return this.cancelBubble;
}
function on() {
  return this.defaultPrevented;
}
l.event = function(n3) {
  return en && (n3 = en(n3)), n3.persist = rn, n3.isPropagationStopped = un, n3.isDefaultPrevented = on, n3.nativeEvent = n3;
};
var ln;
var cn = { enumerable: false, configurable: true, get: function() {
  return this.class;
} };
var fn = l.vnode;
l.vnode = function(n3) {
  "string" == typeof n3.type && (function(n4) {
    var t4 = n4.props, e3 = n4.type, u4 = {}, o4 = -1 === e3.indexOf("-");
    for (var i4 in t4) {
      var l4 = t4[i4];
      if (!("value" === i4 && "defaultValue" in t4 && null == l4 || Q2 && "children" === i4 && "noscript" === e3 || "class" === i4 || "className" === i4)) {
        var c4 = i4.toLowerCase();
        "defaultValue" === i4 && "value" in t4 && null == t4.value ? i4 = "value" : "download" === i4 && true === l4 ? l4 = "" : "translate" === c4 && "no" === l4 ? l4 = false : "o" === c4[0] && "n" === c4[1] ? "ondoubleclick" === c4 ? i4 = "ondblclick" : "onchange" !== c4 || "input" !== e3 && "textarea" !== e3 || X(t4.type) ? "onfocus" === c4 ? i4 = "onfocusin" : "onblur" === c4 ? i4 = "onfocusout" : J2.test(i4) && (i4 = c4) : c4 = i4 = "oninput" : o4 && G2.test(i4) ? i4 = i4.replace(K2, "-$&").toLowerCase() : null === l4 && (l4 = void 0), "oninput" === c4 && u4[i4 = c4] && (i4 = "oninputCapture"), u4[i4] = l4;
      }
    }
    "select" == e3 && u4.multiple && Array.isArray(u4.value) && (u4.value = L(t4.children).forEach(function(n5) {
      n5.props.selected = -1 != u4.value.indexOf(n5.props.value);
    })), "select" == e3 && null != u4.defaultValue && (u4.value = L(t4.children).forEach(function(n5) {
      n5.props.selected = u4.multiple ? -1 != u4.defaultValue.indexOf(n5.props.value) : u4.defaultValue == n5.props.value;
    })), t4.class && !t4.className ? (u4.class = t4.class, Object.defineProperty(u4, "className", cn)) : t4.className && (u4.class = u4.className = t4.className), n4.props = u4;
  })(n3), n3.$$typeof = q3, fn && fn(n3);
};
var an = l.__r;
l.__r = function(n3) {
  an && an(n3), ln = n3.__c;
};
var sn = l.diffed;
l.diffed = function(n3) {
  sn && sn(n3);
  var t4 = n3.props, e3 = n3.__e;
  null != e3 && "textarea" === n3.type && "value" in t4 && t4.value !== e3.value && (e3.value = null == t4.value ? "" : t4.value), ln = null;
};
function bn(n3) {
  return !!n3.__k && (J(null, n3), true);
}
var En = function(n3, t4) {
  return n3(t4);
};

// node_modules/preact/jsx-runtime/dist/jsxRuntime.module.js
var jsxRuntime_module_exports = {};
__export(jsxRuntime_module_exports, {
  Fragment: () => k,
  jsx: () => u3,
  jsxAttr: () => l3,
  jsxDEV: () => u3,
  jsxEscape: () => s3,
  jsxTemplate: () => a3,
  jsxs: () => u3
});
var t3 = /["&<]/;
function n2(r3) {
  if (0 === r3.length || false === t3.test(r3)) return r3;
  for (var e3 = 0, n3 = 0, o4 = "", f4 = ""; n3 < r3.length; n3++) {
    switch (r3.charCodeAt(n3)) {
      case 34:
        f4 = "&quot;";
        break;
      case 38:
        f4 = "&amp;";
        break;
      case 60:
        f4 = "&lt;";
        break;
      default:
        continue;
    }
    n3 !== e3 && (o4 += r3.slice(e3, n3)), o4 += f4, e3 = n3 + 1;
  }
  return n3 !== e3 && (o4 += r3.slice(e3, n3)), o4;
}
var o3 = /acit|ex(?:s|g|n|p|$)|rph|grid|ows|mnc|ntw|ine[ch]|zoo|^ord|itera/i;
var f3 = 0;
var i3 = Array.isArray;
function u3(e3, t4, n3, o4, i4, u4) {
  t4 || (t4 = {});
  var a4, c4, p4 = t4;
  if ("ref" in p4) for (c4 in p4 = {}, t4) "ref" == c4 ? a4 = t4[c4] : p4[c4] = t4[c4];
  var l4 = { type: e3, props: p4, key: n3, ref: a4, __k: null, __: null, __b: 0, __e: null, __c: null, constructor: void 0, __v: --f3, __i: -1, __u: 0, __source: i4, __self: u4 };
  if ("function" == typeof e3 && (a4 = e3.defaultProps)) for (c4 in a4) void 0 === p4[c4] && (p4[c4] = a4[c4]);
  return l.vnode && l.vnode(l4), l4;
}
function a3(r3) {
  var t4 = u3(k, { tpl: r3, exprs: [].slice.call(arguments, 1) });
  return t4.key = t4.__v, t4;
}
var c3 = {};
var p3 = /[A-Z]/g;
function l3(e3, t4) {
  if (l.attr) {
    var f4 = l.attr(e3, t4);
    if ("string" == typeof f4) return f4;
  }
  if (t4 = (function(r3) {
    return null !== r3 && "object" == typeof r3 && "function" == typeof r3.valueOf ? r3.valueOf() : r3;
  })(t4), "ref" === e3 || "key" === e3) return "";
  if ("style" === e3 && "object" == typeof t4) {
    var i4 = "";
    for (var u4 in t4) {
      var a4 = t4[u4];
      if (null != a4 && "" !== a4) {
        var l4 = "-" == u4[0] ? u4 : c3[u4] || (c3[u4] = u4.replace(p3, "-$&").toLowerCase()), s4 = ";";
        "number" != typeof a4 || l4.startsWith("--") || o3.test(l4) || (s4 = "px;"), i4 = i4 + l4 + ":" + a4 + s4;
      }
    }
    return e3 + '="' + n2(i4) + '"';
  }
  return null == t4 || false === t4 || "function" == typeof t4 || "object" == typeof t4 ? "" : true === t4 ? e3 : e3 + '="' + n2("" + t4) + '"';
}
function s3(r3) {
  if (null == r3 || "boolean" == typeof r3 || "function" == typeof r3) return null;
  if ("object" == typeof r3) {
    if (void 0 === r3.constructor) return r3;
    if (i3(r3)) {
      for (var e3 = 0; e3 < r3.length; e3++) r3[e3] = s3(r3[e3]);
      return r3;
    }
  }
  return n2("" + r3);
}
export {
  x as Component,
  k as Fragment,
  N2 as PureComponent,
  P3 as Suspense,
  Q as cloneElement,
  R as createContext,
  _ as createElement,
  $2 as createPortal,
  b as createRef,
  En as flushSync,
  D3 as forwardRef,
  _ as h,
  K as hydrate,
  t as isValidElement,
  jsxRuntime_module_exports as jsxRuntime,
  z3 as lazy,
  M2 as memo,
  l as options,
  J as render,
  x3 as startTransition,
  L as toChildArray,
  bn as unmountComponentAtNode,
  q2 as useCallback,
  x2 as useContext,
  P2 as useDebugValue,
  y2 as useEffect,
  b2 as useErrorBoundary,
  g2 as useId,
  F2 as useImperativeHandle,
  _2 as useLayoutEffect,
  T2 as useMemo,
  h2 as useReducer,
  A2 as useRef,
  d2 as useState
};
