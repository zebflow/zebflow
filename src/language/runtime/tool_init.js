// tool_init.js — globalThis.Tool built-in transform library.
// Pure functions only: no DOM, no state, no side effects.
// Installed once at JsRuntime startup (SSR) and injected into every client bootstrap.
//
// Namespaces: Tool.time  Tool.arr  Tool.stat  Tool.geo
// Locale:    Tool.time.locale('id')  sets global default; per-call override still works

(function () {
  "use strict";

  // ──────────────────────────────────────────────────────────────────────────
  // Global locale state
  // ──────────────────────────────────────────────────────────────────────────
  var _locale = "en";

  // ──────────────────────────────────────────────────────────────────────────
  // Helpers
  // ──────────────────────────────────────────────────────────────────────────
  function pad(n, w) { return String(n).padStart(w || 2, "0"); }

  function toDate(v) {
    if (v instanceof Date) return v;
    if (typeof v === "number") return new Date(v);
    return new Date(v);
  }

  var DAY_NAMES_EN  = ["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"];
  var DAY_NAMES_ID  = ["Minggu","Senin","Selasa","Rabu","Kamis","Jumat","Sabtu"];
  var MON_LONG_EN   = ["January","February","March","April","May","June","July","August","September","October","November","December"];
  var MON_LONG_ID   = ["Januari","Februari","Maret","April","Mei","Juni","Juli","Agustus","September","Oktober","November","Desember"];
  var MON_SHORT_EN  = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
  var MON_SHORT_ID  = ["Jan","Feb","Mar","Apr","Mei","Jun","Jul","Ags","Sep","Okt","Nov","Des"];
  var HIJRI_MONTHS  = ["Muharram","Safar","Rabi'ul Awwal","Rabi'ul Akhir","Jumadil Awwal","Jumadil Akhir","Rajab","Sya'ban","Ramadhan","Syawal","Dzulqa'dah","Dzulhijjah"];

  // ──────────────────────────────────────────────────────────────────────────
  // Tool.time
  // ──────────────────────────────────────────────────────────────────────────
  var time = {

    format: function(date, pattern, locale) {
      var d = toDate(date);
      if (isNaN(d)) return "";
      var id = (locale || _locale) === "id";
      return pattern
        .replace("YYYY", d.getFullYear())
        .replace("YY",   String(d.getFullYear()).slice(-2))
        .replace("MMMM", (id ? MON_LONG_ID  : MON_LONG_EN)[d.getMonth()])
        .replace("MMM",  (id ? MON_SHORT_ID : MON_SHORT_EN)[d.getMonth()])
        .replace("MM",   pad(d.getMonth() + 1))
        .replace(/\bM\b/, d.getMonth() + 1)
        .replace("dddd", (id ? DAY_NAMES_ID : DAY_NAMES_EN)[d.getDay()])
        .replace("ddd",  (id ? DAY_NAMES_ID : DAY_NAMES_EN)[d.getDay()].slice(0, 3))
        .replace("DD",   pad(d.getDate()))
        .replace(/\bD\b/, d.getDate())
        .replace("HH",   pad(d.getHours()))
        .replace(/\bH\b/, d.getHours())
        .replace("hh",   pad(d.getHours() % 12 || 12))
        .replace("mm",   pad(d.getMinutes()))
        .replace("ss",   pad(d.getSeconds()))
        .replace(/\bA\b/, d.getHours() < 12 ? "AM" : "PM")
        .replace(/\ba\b/, d.getHours() < 12 ? "am" : "pm");
    },

    diff: function(a, b, unit) {
      var da = toDate(a), db = toDate(b);
      var ms = db.getTime() - da.getTime();
      if (unit === "second" || unit === "seconds") return Math.floor(ms / 1000);
      if (unit === "minute" || unit === "minutes") return Math.floor(ms / 60000);
      if (unit === "hour"   || unit === "hours")   return Math.floor(ms / 3600000);
      if (unit === "day"    || unit === "days")     return Math.floor(ms / 86400000);
      if (unit === "week"   || unit === "weeks")    return Math.floor(ms / 604800000);
      if (unit === "month"  || unit === "months") {
        return (db.getFullYear() - da.getFullYear()) * 12 + (db.getMonth() - da.getMonth());
      }
      if (unit === "year"   || unit === "years") {
        return db.getFullYear() - da.getFullYear();
      }
      return ms;
    },

    add: function(date, n, unit) {
      var d = new Date(toDate(date).getTime());
      if (unit === "second" || unit === "seconds") d.setSeconds(d.getSeconds() + n);
      else if (unit === "minute" || unit === "minutes") d.setMinutes(d.getMinutes() + n);
      else if (unit === "hour"   || unit === "hours")   d.setHours(d.getHours() + n);
      else if (unit === "day"    || unit === "days")    d.setDate(d.getDate() + n);
      else if (unit === "week"   || unit === "weeks")   d.setDate(d.getDate() + n * 7);
      else if (unit === "month"  || unit === "months")  d.setMonth(d.getMonth() + n);
      else if (unit === "year"   || unit === "years")   d.setFullYear(d.getFullYear() + n);
      return d;
    },

    subtract: function(date, n, unit) {
      return time.add(date, -n, unit);
    },

    startOf: function(date, unit) {
      var d = new Date(toDate(date).getTime());
      if (unit === "day")   { d.setHours(0, 0, 0, 0); }
      if (unit === "week")  { d.setDate(d.getDate() - d.getDay()); d.setHours(0, 0, 0, 0); }
      if (unit === "month") { d.setDate(1); d.setHours(0, 0, 0, 0); }
      if (unit === "year")  { d.setMonth(0, 1); d.setHours(0, 0, 0, 0); }
      return d;
    },

    endOf: function(date, unit) {
      var d = new Date(toDate(date).getTime());
      if (unit === "day")   { d.setHours(23, 59, 59, 999); }
      if (unit === "month") { d.setMonth(d.getMonth() + 1, 0); d.setHours(23, 59, 59, 999); }
      if (unit === "year")  { d.setMonth(11, 31); d.setHours(23, 59, 59, 999); }
      return d;
    },

    isBefore: function(a, b) { return toDate(a).getTime() < toDate(b).getTime(); },
    isAfter:  function(a, b) { return toDate(a).getTime() > toDate(b).getTime(); },
    isSame:   function(a, b, unit) {
      if (!unit) return toDate(a).getTime() === toDate(b).getTime();
      return time.startOf(a, unit).getTime() === time.startOf(b, unit).getTime();
    },

    relativeTime: function(date, locale) {
      var d = toDate(date);
      var diff = Date.now() - d.getTime();
      var abs = Math.abs(diff);
      var future = diff < 0;
      var id = (locale || _locale) === "id";
      var units = [
        [60,          id ? "detik"  : "second"],
        [3600,        id ? "menit"  : "minute"],
        [86400,       id ? "jam"    : "hour"],
        [2592000,     id ? "hari"   : "day"],
        [31536000,    id ? "bulan"  : "month"],
        [Infinity,    id ? "tahun"  : "year"],
      ];
      var secs = abs / 1000;
      var prev = 1;
      for (var i = 0; i < units.length; i++) {
        var limit = units[i][0], label = units[i][1];
        if (secs < limit) {
          var val = Math.round(secs / prev);
          var s = !id && val !== 1 ? "s" : "";
          return future
            ? (id ? "dalam " + val + " " + label : "in " + val + " " + label + s)
            : (id ? val + " " + label + " lalu"  : val + " " + label + s + " ago");
        }
        prev = limit;
      }
    },

    tz: function(date, timezone) {
      // Returns a new Date adjusted to the given IANA timezone.
      // Works via Intl in browser and V8/Deno. Returns original if Intl unavailable.
      var d = toDate(date);
      if (typeof Intl === "undefined" || !Intl.DateTimeFormat) return d;
      try {
        var fmt = new Intl.DateTimeFormat("en-US", {
          timeZone: timezone,
          year: "numeric", month: "2-digit", day: "2-digit",
          hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false,
        });
        var parts = fmt.formatToParts(d).reduce(function(acc, p) {
          acc[p.type] = p.value; return acc;
        }, {});
        return new Date(
          parts.year + "-" + parts.month + "-" + parts.day + "T" +
          (parts.hour === "24" ? "00" : parts.hour) + ":" + parts.minute + ":" + parts.second
        );
      } catch (e) { return d; }
    },

    toHijri: function(date) {
      var d = toDate(date);
      var jd = Math.floor(d.getTime() / 86400000) + 2440587.5;
      jd = Math.floor(jd);
      var l  = jd - 1948440 + 10632;
      var n  = Math.floor((l - 1) / 10631);
      l = l - 10631 * n + 354;
      var j = (Math.floor((10985 - l) / 5316)) * (Math.floor((50 * l) / 17719)) +
              (Math.floor(l / 5670))           * (Math.floor((43 * l) / 15238));
      l = l - (Math.floor((30 - j) / 15)) * (Math.floor((17719 * j) / 50)) -
              (Math.floor(j / 16))          * (Math.floor((15238 * j) / 43)) + 29;
      var month = Math.floor((24 * l) / 709);
      var day   = l - Math.floor((709 * month) / 24);
      var year  = 30 * n + j - 30;
      return { day: day, month: month, year: year, monthName: HIJRI_MONTHS[month - 1] || "" };
    },

    fromHijri: function(hDay, hMonth, hYear) {
      var jd = Math.floor((11 * hYear + 3) / 30) + 354 * hYear + 30 * hMonth -
               Math.floor((hMonth - 1) / 2) + hDay + 1948440 - 385;
      return new Date((jd - 2440587.5) * 86400000);
    },

    locale: function(code) { _locale = code; return time; },
  };

  // ──────────────────────────────────────────────────────────────────────────
  // Tool.arr
  // ──────────────────────────────────────────────────────────────────────────
  var arr = {

    sortBy: function(data, key, dir) {
      if (!Array.isArray(data)) return data;
      var d = dir === "desc" ? -1 : 1;
      return data.slice().sort(function(a, b) {
        var av = typeof key === "function" ? key(a) : a[key];
        var bv = typeof key === "function" ? key(b) : b[key];
        if (av == null) return d;
        if (bv == null) return -d;
        if (typeof av === "string") return d * av.localeCompare(String(bv));
        return d * (av - bv);
      });
    },

    filterBy: function(data, filters) {
      if (!Array.isArray(data)) return data;
      if (typeof filters === "function") return data.filter(filters);
      if (typeof filters === "string") {
        var q = filters.toLowerCase();
        return data.filter(function(item) {
          return Object.values(item).some(function(v) {
            return String(v).toLowerCase().includes(q);
          });
        });
      }
      return data.filter(function(item) {
        return Object.entries(filters).every(function(kv) {
          return item[kv[0]] === kv[1];
        });
      });
    },

    paginate: function(data, page, size) {
      if (!Array.isArray(data)) return { items: [], total: 0, totalPages: 0, page: page };
      var total = data.length;
      var totalPages = Math.max(1, Math.ceil(total / size));
      var p = Math.max(1, Math.min(page, totalPages));
      return { items: data.slice((p - 1) * size, p * size), total: total, totalPages: totalPages, page: p };
    },

    groupBy: function(data, key) {
      if (!Array.isArray(data)) return {};
      return data.reduce(function(acc, item) {
        var k = typeof key === "function" ? key(item) : item[key];
        if (!acc[k]) acc[k] = [];
        acc[k].push(item);
        return acc;
      }, {});
    },

    flatGroupBy: function(data, key) {
      var groups = arr.groupBy(data, key);
      return Object.entries(groups).map(function(kv) {
        var obj = {}; obj[typeof key === "string" ? key : "_key"] = kv[0]; obj.items = kv[1];
        return obj;
      });
    },

    sumBy: function(data, key) {
      if (!Array.isArray(data)) return 0;
      return data.reduce(function(s, item) { return s + (Number(typeof key === "function" ? key(item) : item[key]) || 0); }, 0);
    },

    countBy: function(data, key) {
      if (!Array.isArray(data)) return {};
      return data.reduce(function(acc, item) {
        var k = typeof key === "function" ? key(item) : item[key];
        acc[k] = (acc[k] || 0) + 1;
        return acc;
      }, {});
    },

    uniqueBy: function(data, key) {
      if (!Array.isArray(data)) return data;
      var seen = new Set();
      return data.filter(function(item) {
        var k = typeof key === "function" ? key(item) : item[key];
        if (seen.has(k)) return false;
        seen.add(k); return true;
      });
    },
  };

  // ──────────────────────────────────────────────────────────────────────────
  // Tool.stat
  // ──────────────────────────────────────────────────────────────────────────
  var stat = {

    mean: function(a) {
      if (!a || !a.length) return 0;
      return a.reduce(function(s, v) { return s + v; }, 0) / a.length;
    },

    median: function(a) {
      if (!a || !a.length) return 0;
      var s = a.slice().sort(function(x, y) { return x - y; });
      var m = Math.floor(s.length / 2);
      return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
    },

    variance: function(a) {
      if (!a || a.length < 2) return 0;
      var m = stat.mean(a);
      return a.reduce(function(s, v) { return s + Math.pow(v - m, 2); }, 0) / a.length;
    },

    stddev: function(a) {
      return Math.sqrt(stat.variance(a));
    },

    percentile: function(a, p) {
      if (!a || !a.length) return 0;
      var s = a.slice().sort(function(x, y) { return x - y; });
      var idx = (p / 100) * (s.length - 1);
      var lo = Math.floor(idx), hi = Math.ceil(idx);
      return s[lo] + (s[hi] - s[lo]) * (idx - lo);
    },

    zscore: function(a) {
      var m = stat.mean(a), s = stat.stddev(a) || 1;
      return a.map(function(v) { return (v - m) / s; });
    },

    rateAbove: function(values, threshold) {
      if (!values || !values.length) return 0;
      return (values.filter(function(v) { return v >= threshold; }).length / values.length) * 100;
    },

    correlation: function(xs, ys) {
      var n = Math.min(xs.length, ys.length);
      if (n < 2) return 0;
      var mx = stat.mean(xs), my = stat.mean(ys);
      var num = 0, dx = 0, dy = 0;
      for (var i = 0; i < n; i++) {
        var ex = xs[i] - mx, ey = ys[i] - my;
        num += ex * ey; dx += ex * ex; dy += ey * ey;
      }
      return dx && dy ? num / Math.sqrt(dx * dy) : 0;
    },

    linreg: function(xs, ys) {
      var n = Math.min(xs.length, ys.length);
      if (n < 2) return { slope: 0, intercept: 0, r2: 0 };
      var mx = stat.mean(xs), my = stat.mean(ys);
      var num = 0, den = 0;
      for (var i = 0; i < n; i++) { var ex = xs[i] - mx; num += ex * (ys[i] - my); den += ex * ex; }
      var slope = den ? num / den : 0;
      var intercept = my - slope * mx;
      var r = stat.correlation(xs, ys);
      return { slope: slope, intercept: intercept, r2: r * r };
    },

    histogram: function(a, bins) {
      if (!a || !a.length) return [];
      var min = Math.min.apply(null, a), max = Math.max.apply(null, a);
      var width = (max - min) / bins || 1;
      var counts = new Array(bins).fill(0);
      a.forEach(function(v) {
        var i = Math.min(Math.floor((v - min) / width), bins - 1);
        counts[i]++;
      });
      return counts.map(function(count, i) {
        return { min: min + i * width, max: min + (i + 1) * width, count: count };
      });
    },
  };

  // ──────────────────────────────────────────────────────────────────────────
  // Tool.geo
  // ──────────────────────────────────────────────────────────────────────────
  var geo = {

    centroid: function(polygon) {
      if (!polygon || !polygon.length) return [0, 0];
      var x = 0, y = 0, n = polygon.length;
      for (var i = 0; i < n; i++) { x += polygon[i][0]; y += polygon[i][1]; }
      return [x / n, y / n];
    },

    distance: function(a, b) {
      // Haversine — returns metres
      var R = 6371000;
      var lat1 = a[1] * Math.PI / 180, lat2 = b[1] * Math.PI / 180;
      var dLat = (b[1] - a[1]) * Math.PI / 180;
      var dLng = (b[0] - a[0]) * Math.PI / 180;
      var h = Math.sin(dLat / 2) * Math.sin(dLat / 2) +
              Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLng / 2) * Math.sin(dLng / 2);
      return R * 2 * Math.atan2(Math.sqrt(h), Math.sqrt(1 - h));
    },

    pointInPolygon: function(point, polygon) {
      var x = point[0], y = point[1], inside = false;
      for (var i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
        var xi = polygon[i][0], yi = polygon[i][1];
        var xj = polygon[j][0], yj = polygon[j][1];
        if (((yi > y) !== (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi)) {
          inside = !inside;
        }
      }
      return inside;
    },

    bbox: function(features) {
      var minLng = Infinity, minLat = Infinity, maxLng = -Infinity, maxLat = -Infinity;
      function absorb(c) {
        if (c[0] < minLng) minLng = c[0]; if (c[0] > maxLng) maxLng = c[0];
        if (c[1] < minLat) minLat = c[1]; if (c[1] > maxLat) maxLat = c[1];
      }
      (features || []).forEach(function(f) {
        if (Array.isArray(f) && typeof f[0] === "number") { absorb(f); }
        else if (f && f.geometry) {
          var coords = f.geometry.coordinates;
          (Array.isArray(coords[0]) ? coords.flat(Infinity) : [coords])
            .filter(function(_, i) { return i % 2 === 0; })
            .forEach(function(_, i, flat) { absorb([flat[i * 2], flat[i * 2 + 1]]); });
        }
      });
      return [minLng, minLat, maxLng, maxLat];
    },
  };

  // ──────────────────────────────────────────────────────────────────────────
  // Install
  // ──────────────────────────────────────────────────────────────────────────
  globalThis.Tool = { time: time, arr: arr, stat: stat, geo: geo };
})();
