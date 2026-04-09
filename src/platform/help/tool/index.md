# Tool.* Globals

`Tool` is a pure-function utility library available in TSX templates and `n.script` nodes. No DOM, no state, no side effects. Four namespaces: `Tool.time`, `Tool.arr`, `Tool.stat`, `Tool.geo`.

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

Units: `day`, `week`, `month`, `year`

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

---

## Tool.arr

Array utilities.

```js
Tool.arr.groupBy(items, 'category')         // → { [key]: items[] }
Tool.arr.sortBy(items, 'name')              // ascending sort by field
Tool.arr.sortBy(items, 'date', 'desc')      // descending
Tool.arr.unique(arr)                         // deduplicate
Tool.arr.unique(items, 'id')                // deduplicate by field
Tool.arr.chunk(arr, 10)                     // → [[...], [...], ...]
Tool.arr.flatten(nested)                    // deep flatten
Tool.arr.sum(items, 'amount')               // sum numeric field
Tool.arr.min(items, 'price')                // min by field
Tool.arr.max(items, 'price')                // max by field
Tool.arr.avg(items, 'score')                // average
```

---

## Tool.stat

Statistics and number formatting.

```js
Tool.stat.round(3.14159, 2)          // 3.14
Tool.stat.percent(45, 200)           // 22.5
Tool.stat.currency(1500000, 'IDR')   // "Rp 1.500.000"
Tool.stat.currency(1234.56, 'USD')   // "$1,234.56"
Tool.stat.format(1234567, ',')       // "1,234,567"
Tool.stat.clamp(value, min, max)     // clamp to range
Tool.stat.lerp(0, 100, 0.5)          // 50
```

---

## Tool.geo

Geographic utilities.

```js
Tool.geo.distance([lon1, lat1], [lon2, lat2])              // → distance in km (haversine)
Tool.geo.distance(lat1, lon1, lat2, lon2)                  // → legacy form, also km
Tool.geo.bbox(pointsOrFeatures)                            // → [minLon, minLat, maxLon, maxLat]
Tool.geo.center(pointsOrFeatures)                          // → [lon, lat]
Tool.geo.pointInPolygon([lon, lat], polygonOrMultiPolygon) // → boolean
Tool.geo.centroid(polygonOrMultiPolygon)                   // → [lon, lat]
Tool.geo.nearestPoint([lon, lat], points)                  // → { index, distance }
```

`pointInPolygon` and `centroid` accept GeoJSON `Polygon` / `MultiPolygon`.
Simple polygon ring arrays like `[[lon, lat], ...]` are also accepted for backward compatibility.

---

## Usage Examples

### In TSX templates

```tsx
export default function PostList(input) {
  const state = usePageState(input.state ?? { posts: [] });
  const grouped = Tool.arr.groupBy(state.posts, 'category');
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
  amount_display: Tool.stat.currency(r.amount, 'IDR'),
}));
return { rows: formatted };
```
