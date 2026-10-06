/**
 * The Potoru story the specs author and play: compiled in the browser by
 * `PotoEditor` (`zeb/potoru`), then uploaded and played from the project.
 */

/** A Potoru format v4 source folder: a red square crossing a blue stage. */
export const STORY: Record<string, string> = {
  "potoru.lock.yml": "lockVersion: 1\npackages: []\n",
  "potoru.project.yml": [
    "format: potoru", "kind: story", "assets: []", "composition:", "  id: composition-main", '  name: "Square"',
    "entry: scene-main", "metadata:", "  id: square", '  name: "Square"', '  updatedAt: "2000-01-01T00:00:00.000Z"',
    "objects: []", "requires:", "  - vector2d", "  - timeline", "  - tick-time", "scenes:", "  - scene-main",
    "timebase:", "  tickRate: 1000", "versions:", "  runtime: 1", "  schema: 5", "  scoreLang: 1", "",
  ].join("\n"),
  "scenes/scene-main/scene.yml": [
    "name: Square", "durationTicks: 2000", "fps: 30", "root: inline", "stage:", "  width: 320", "  height: 180",
    "  background:", "    kind: color", '    color: "#0000ff"', "  clipContent: true", "",
  ].join("\n"),
  "scenes/scene-main/root/object.yml": [
    "initialState: main", "parts:", "  - id: main-box", "    name: box", "    x: 20", "    y: 60", "    width: 60",
    "    height: 60",
    '    drawable: {kind: vector, geometry: {kind: rect}, paint: {fill: "#ff0000", stroke: transparent, strokeWidth: 0}}',
    "states:", "  - main", "",
  ].join("\n"),
  "scenes/scene-main/root/states/main.yml": [
    "name: main", 'summary: ""', "durationTicks: 2000", "fps: 30", "tracks:", "  - id: main-box-x",
    "    label: main-box.x", "    kind: property", "    targetId: main-box", "    clips:",
    "      - {id: main-box-x-1, label: x, property: x, startTicks: 0, durationTicks: 2000, easing: linear, from: 20, playback: once, to: 240, tone: blue}",
    "",
  ].join("\n"),
};
