# Maps

Zebflow can inspect spatial files, convert them, publish map layers, and serve
map requests from project data.

## Normal Flow

```text
upload -> inspect -> choose layer -> convert -> publish -> query or display
```

Inspection matters because one source file can contain several layers. A
FileGDB, for example, can list many named layers. Select the intended layer
before conversion.

## Geometry

Do not guess which column contains geometry. Source metadata should identify it.
Conversion should preserve or clearly normalize that meaning. Downstream map
code should use metadata, not a fixed list of guessed column names.

## Formats

Current spatial work includes GeoJSON and GeoParquet, plus source formats
supported by the geo conversion engine. Map output includes GeoJSON, raster map
images, and vector tile paths where supported by the selected source.

## Styles

Server styles can set flat colors and data driven categories or numeric ranges.
Use a palette supported by the map style definition. An unknown palette should
produce a clear validation error instead of a silent fallback.

## Large Layers

Large spatial files should stay as files. Query only the required bounding box,
columns, or sample. Expensive sorting and full scans need memory limits and a
spill path. Do not load an entire large layer into a normal JSON pipeline value.
