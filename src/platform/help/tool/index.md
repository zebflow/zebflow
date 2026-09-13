# Tool.* Globals

`Tool` is a pure-function utility library available in TSX templates and `n.script` nodes. No DOM, no state, no side effects. Five namespaces: `Tool.time`, `Tool.arr`, `Tool.stat`, `Tool.csv`, `Tool.geo`.

Available as `globalThis.Tool` — use it directly, no import needed.

---

## Tool.time

Date/time formatting and arithmetic.

### Locale

```js
Tool.time.locale('id');   // Set global default to Indonesian
Tool.time.locale('en');   // Back to English
```

### format(date, pattern, locale?)

```js
Tool.time.format(new Date(), 'YYYY-MM-DD')         // "2025-03-25"
Tool.time.format(new Date(), 'DD MMMM YYYY', 'id') // "25 Maret 2025"
Tool.time.format(new Date(), 'HH:mm:ss')           // "14:30:05"
```

Pattern tokens: `YYYY`, `YY`, `MMMM`, `MMM`, `MM`, `M`, `dddd`, `ddd`, `DD`, `D`, `HH`, `H`, `hh`, `mm`, `ss`, `A`, `a`

### diff(a, b, unit)

```js
Tool.time.diff('2025-01-01', '2025-03-25', 'days')   // 83
Tool.time.diff(start, end, 'months')
Tool.time.diff(start, end, 'hours')
```

Units: `second(s)`, `minute(s)`, `hour(s)`, `day(s)`, `week(s)`, `month(s)`, `year(s)`

### add / subtract

```js
Tool.time.add(new Date(), 7, 'days')          // 7 days from now
Tool.time.subtract(new Date(), 1, 'months')   // 1 month ago
```

### startOf / endOf

```js
Tool.time.startOf(new Date(), 'month')  // first moment of current month
Tool.time.endOf(new Date(), 'year')     // last moment of current year
```

Units: `day`, `week`, `month`, `year` for `startOf`; `endOf` knows `day`, `month`, `year`.

### Comparisons

```js
Tool.time.isBefore(a, b)          // → boolean
Tool.time.isAfter(a, b)           // → boolean
Tool.time.isSame(a, b)            // exact timestamp equality
Tool.time.isSame(a, b, 'day')     // same day
```

### relativeTime(date, locale?)

```js
Tool.time.relativeTime(new Date(Date.now() - 3600000))  // "1 hour ago"
Tool.time.relativeTime(pastDate, 'id')                  // "1 jam lalu"
```

### tz(date, timezone)

```js
Tool.time.tz(new Date(), 'Asia/Jakarta')  // → Date adjusted for timezone
```

### Hijri

```js
Tool.time.toHijri(new Date())             // → { day, month, year, monthName }
Tool.time.fromHijri(1, 9, 1447)           // → Date (day, month, year)
```

---

## Tool.arr

Array utilities for rows. `key` is a field name or a function of the item.

```js
Tool.arr.sortBy(rows, 'name')                 // ascending; strings by locale, nulls last
Tool.arr.sortBy(rows, 'created_at', 'desc')
Tool.arr.filterBy(rows, { status: 'active' }) // every listed field equals
Tool.arr.filterBy(rows, 'quarterly')          // a string: case-insensitive match in any field
Tool.arr.filterBy(rows, (r) => r.score > 80)  // a predicate
Tool.arr.paginate(rows, 2, 20)                // → { items, total, totalPages, page }
Tool.arr.groupBy(rows, 'category')            // → { [key]: rows[] }
Tool.arr.flatGroupBy(rows, 'category')        // → [{ key, items }, …]
Tool.arr.sumBy(rows, 'amount')                // number
Tool.arr.countBy(rows, 'status')              // → { [key]: count }
Tool.arr.uniqueBy(rows, 'email')              // first occurrence wins
```

Nothing else — `unique`, `chunk`, `flatten`, `min`, `max`, `avg` do not exist;
use the array methods or `Tool.stat`.

---

## Tool.stat

Statistics over arrays of numbers.

```js
Tool.stat.mean(values)            Tool.stat.median(values)
Tool.stat.variance(values)        Tool.stat.stddev(values)
Tool.stat.percentile(values, 90)  Tool.stat.zscore(values)       // → array of z-scores
Tool.stat.rateAbove(values, 80)   // percentage of values >= threshold
Tool.stat.correlation(xs, ys)     Tool.stat.linreg(xs, ys)       // → { slope, intercept, r2 }
Tool.stat.histogram(values, 10)   // → [{ min, max, count }, …]
```

Number formatting (`round`, `percent`, `currency`, `format`) is not here —
use `Intl.NumberFormat` and `toFixed`.

---

## Tool.csv

CSV parsing and serialization.

```js
Tool.csv.parse("id,name\n1,Alice\n2,Bob")
Tool.csv.stringify([{ id: 1, name: "Alice" }, { id: 2, name: "Bob" }])
```

`Tool.csv.parse(text, options?)`
- `delimiter` — defaults to `,`
- `header` — defaults to `true`

`Tool.csv.stringify(rows, options?)`
- `delimiter` — defaults to `,`
- `header` — defaults to `true`
- `columns` — optional explicit column order

---

## Tool.geo

Geographic utilities.

```js
Tool.geo.distance([lon1, lat1], [lon2, lat2])              // → distance in km (haversine)
Tool.geo.distance(lat1, lon1, lat2, lon2)                  // → legacy form, also km
Tool.geo.bbox(pointsOrFeatures)                            // → [minLon, minLat, maxLon, maxLat]
Tool.geo.center(pointsOrFeatures)                          // → [lon, lat]
Tool.geo.pointInPolygon([lon, lat], polygonOrMultiPolygon) // → boolean
Tool.geo.booleanPointInPolygon([lon, lat], polygon)        // → boolean (turf-style alias)
Tool.geo.centroid(polygonOrMultiPolygon)                   // → [lon, lat]
Tool.geo.nearestPoint([lon, lat], points)                  // → { index, distance, point }
Tool.geo.parseWktLineString("LINESTRING (...)")            // → [[lon, lat], ...]
Tool.geo.heading([lon1, lat1], [lon2, lat2])              // → degrees
Tool.geo.bearing([lon1, lat1], [lon2, lat2])              // → 0..360 geographic bearing
Tool.geo.routeProgress(route)                             // → { totalDistance, distances, segments }
Tool.geo.interpolateRoute(route, 0.5)                     // → [lon, lat]
```

`pointInPolygon` and `centroid` accept GeoJSON `Polygon` / `MultiPolygon`.
Simple polygon ring arrays like `[[lon, lat], ...]` are also accepted for backward compatibility.

---

## Usage Examples

### In TSX templates

```tsx
export default function PostList(input) {
  const grouped = Tool.arr.groupBy(input.rows ?? [], 'category');
  return (
    <div>
      {Object.entries(grouped).map(([cat, posts]) => (
        <section key={cat}>
          <h2>{cat}</h2>
          {posts.map(p => (
            <div key={p.id}>
              <span>{p.title}</span>
              <time>{Tool.time.format(p.created_at, 'DD MMM YYYY')}</time>
              <span>{Tool.time.relativeTime(p.created_at)}</span>
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}
```

### In n.script nodes

```js
// Node body
const formatted = input.rows.map(r => ({
  ...r,
  date_label: Tool.time.format(r.created_at, 'DD MMMM YYYY', 'id'),
  amount_display: new Intl.NumberFormat('id-ID', { style: 'currency', currency: 'IDR' }).format(r.amount),
}));
return { rows: formatted };
```
