# Data licences

aslnut ships **code only**. No tiles, heights, packs or basemaps are in the npm package. The provider factories
point at services run by others, each with its own terms. aslnut carries those terms as **capabilities** on each
source (`terms: { show, prefetch, keep, export }`), and the loader enforces them.

| factory | service | licence / terms | show | prefetch | keep | export | attribution shown |
|---|---|---|---|---|---|---|---|
| `imagery.esri()` | Esri World Imagery (ArcGIS Online) | Esri terms of use; a licence is needed beyond light, non-commercial viewing | yes | no | no | no | Imagery © Esri, Maxar, Earthstar Geographics |
| `imagery.osm()` | OpenStreetMap standard tiles (`tile.openstreetmap.org`) | data ODbL; the tile service's usage policy forbids bulk downloading, prefetching and offline use | yes | no | no | no | © OpenStreetMap contributors |
| `imagery.sentinel()` | EOX Sentinel-2 cloudless (s2cloudless), by EOX IT Services | CC BY 4.0 (the 2016 mosaic is CC BY-NC-SA; aslnut uses a CC BY year by default) | yes | yes | yes | yes | Sentinel-2 cloudless by EOX IT Services GmbH (contains modified Copernicus Sentinel data) |
| `terrain.mapzen()` | Terrain tiles (Terrarium) on AWS Open Data, by Mapzen | open data; several sources, each with its own attribution | yes | yes | yes | yes | Terrain: Mapzen terrain tiles (AWS Open Data) |
| `imagery.custom()`, `terrain.grid()`, `terrain.custom()` | yours | whatever you set in `terms` and `attribution` | default yes | default yes | default yes | default yes | yours |
| `earthLite({ url })` | an offline basemap pack you host | yours: build it only from data whose licence allows redistribution | yes | yes | yes | yes | from the pack |

**Show only** (Esri, OSM) means: drawn while you look, kept in memory for the page's life, never written to a cache
scope, never prefetched for views you have not reached, never exported into a pack or a capture.

Change a source's terms only when you hold the rights to, for example a commercial Esri licence:
`imagery.esri({ terms: { keep: true } })`.

**Small tables in the code.** `zoneLookups.nearest()` (core › time › zones) carries the time zones' principal places
from the IANA time zone database (`zone1970.tab`, public domain), loaded only when a world asks for its zone from the
place.

**Stars.** `catalogues.hipparcos()` (modules › world › earth › air › sky › stars) asks CDS VizieR for the Hipparcos main
catalogue (ESA 1997, I/239) down to the magnitude shown; VizieR's terms apply, and the credit "Stars: Hipparcos (ESA
1997), CDS VizieR" is shown while the stars are. A list you serve yourself (`tools/stars-pack.mjs`) carries the same.
