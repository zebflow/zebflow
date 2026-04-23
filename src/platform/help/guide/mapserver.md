# MapServer

MapServer is Zebflow's first-class geospatial publishing and serving surface.

Its job is not only storing geodata. Its job is:

- publishing layers
- resolving spatial requests efficiently
- serving map-facing responses

## What it pairs with

- project files such as GeoJSON
- future chunked/published artifacts
- TSX pages using `zeb/deckgl`

## Why it matters

It turns geospatial data into project-native application behavior.

Typical flow:

1. publish a layer
2. query it by viewport / filters
3. render it in a page or map experience

MapServer is treated as a first-class capability in Zebflow, not as a plugin.
