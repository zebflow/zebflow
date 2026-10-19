//! `world.roads.bake` — a place's road pack, designed on the server.
//!
//! Runs aslnut's Rust road baker (the `baker` crate, Apache-2.0) inside this
//! process, on one core: it draws each street from its map line (a fair curve,
//! a cross-section from its tags and the rules, junction patterns), cuts the
//! place into one planar partition, gives it heights over the ground and writes
//! the pack a `zeb/world` page reads with `bakedRoads({ url })`.
//!
//! `--from` is the store folder holding the seed, `--place` the file stem of
//! its files: `<place>.geojson` (road lines with their `nodes` and tags),
//! `<place>.ground.json` and `<place>.ground.f32` (the ground grid) and, where
//! there, `<place>.paths.geojson`, `<place>.reserve.geojson` and
//! `<place>.water.geojson`. `--rules` is a store key of a JSON file whose
//! fields override the baker's rules, field by field.
//!
//! The baker writes each `.json` with a `.json.gz` beside it; only the plain
//! `.json` files are stored, because a page asks for `.json`. They go under
//! `--folder` (default `world/roads/<place>`), `index.json` last, so a folder
//! without one is a bake that did not finish.
//!
//! The answer is one key, `roads: { folder, items, count, index, report }`;
//! the rest of the payload stays.

use std::sync::Arc;
use std::time::Instant;

use aslnut_baker::{rules::Rules, seed::SeedFiles};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::project_store::{NodeStore, OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::{MAX_SCRATCH_BYTES, MAX_SCRATCH_FILES};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFailureSemantic, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::ZebFsEntryKind;

pub const NODE_KIND: &str = "world.roads.bake";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// The scratch of the bake, the baker task and the store: the world's side.
pub const CODE: &str = "FW_NODE_WORLD_ROADS_BAKE";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_WORLD_ROADS_BAKE_CONFIG";
/// A seed file (or the rules file) is missing, not readable, or over the ceiling.
const SOURCE_CODE: &str = "FW_NODE_WORLD_ROADS_BAKE_SOURCE";
/// The baker refused what the seed files hold.
const SEED_CODE: &str = "FW_NODE_WORLD_ROADS_BAKE_SEED";

const DEFAULT_FOLDER_ROOT: &str = "world/roads";
const MAX_PLACE_LEN: usize = 64;
/// All the seed files together; each is also under the store's own read cap.
const MAX_SEED_BYTES: u64 = 256 * 1024 * 1024;
/// How far the ground grid may reach on a side. The design covers the whole
/// grid in 500 m tiles; a place wider than this is baked in parts.
const MAX_EXTENT_M: f64 = 50_000.0;

/// The seed files of a place: (suffix after `<place>`, required).
const SEED_FILES: &[(&str, bool)] =
    &[(".geojson", true), (".ground.json", true), (".ground.f32", true), (".paths.geojson", false), (".reserve.geojson", false), (".water.geojson", false)];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The store folder holding the seed (or a tree answer, `{ folder }`).
    #[serde(default)]
    pub from: Value,
    /// The seed's file stem: letters, digits, `-` and `_`.
    #[serde(default)]
    pub place: String,
    /// A store key (or FileRef) of a rules JSON file; empty: the baker's rules.
    #[serde(default)]
    pub rules: Value,
    /// The store the seed is read from and the pack written to.
    #[serde(default)]
    pub store: Option<String>,
    /// Destination folder (default `world/roads/<place>`).
    #[serde(default)]
    pub folder: String,
    /// `error` (default, when the folder is not empty), `skip` or `overwrite`.
    #[serde(default)]
    pub on_conflict: Option<String>,
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

pub fn definition() -> NodeDefinition {
    let index = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "world/packs/roads/site-a/index.json", "filename": "index.json", "mime": "application/json", "kind": "json", "size": 6120, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Roads Bake".to_string(),
        description: "Bakes a place's road pack on the server with aslnut's Rust road baker (Apache-2.0), in this process, on one core. \
            `--from` is the store folder holding the seed and `--place` the file stem of its files: `<place>.geojson` (OpenStreetMap road lines, each with its `nodes` and tags), \
            `<place>.ground.json` and `<place>.ground.f32` (the ground grid), and where there `<place>.paths.geojson`, `<place>.reserve.geojson` and `<place>.water.geojson`. \
            `--rules` is a store key of a JSON file whose fields override the baker's rules (street classes, widths, kerbs, corners); its defaults are one city's street standards, so give your own. \
            Writes the pack under `--folder` (default `world/roads/<place>`): `index.json`, a tile per 500 m (`<ix>_<iz>.json`), `clamp.json` with `clamp/<ix>_<iz>.json`, `terrain.json` and `walk.json` — \
            the plain `.json` files only, not the `.json.gz` the baker writes beside each, because a page asks for `.json`. \
            A page reads the pack with `bakedRoads({ url: '<folder>/index.json' })` and `clampTerrain(terrain, '<folder>/clamp.json')` from `zeb/world`. \
            The pack is built from OpenStreetMap data (ODbL, © OpenStreetMap contributors) and records its sources and credits in `index.json`; the world draws them, so keep them on screen. \
            Today the seed comes from aslnut's own tools (`packages/aslnut/tools/roads/extract.py` for the road lines, `roads/dem.py` for the ground). \
            The seed is read from and the pack written to `--store`; it is refused over 256 MiB, a ground grid wider than 50 km, or a pack over 2000 files or 256 MiB. \
            A big place takes time and memory on one core: raise `--timeout` (default 30s). A folder that is not empty is an error unless `--on-conflict` says otherwise. \
            Adds `roads: { folder, items, count, index, report }` — `items` a durable FileRef per file written, `index` the FileRef of `index.json`, `report` the baker's summary \
            (streets, junctions, heights, partition, mesh, gates, tiles, bytes) — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "roads": {
                    "type": "object",
                    "properties": {
                        "folder": { "type": "string" },
                        "items": { "type": "array", "description": "A durable FileRef per file written" },
                        "count": { "type": "integer" },
                        "index": { "type": "object", "description": "The FileRef of index.json (the pack's entry); null when a skipped bake finds none" },
                        "report": { "type": "object", "description": "The baker's summary; null when the bake was skipped" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--from", "from", "The store folder holding the seed: <place>.geojson, <place>.ground.json, <place>.ground.f32 and optionally <place>.paths.geojson, <place>.reserve.geojson, <place>.water.geojson.", "text") },
            DslFlag { required: true, ..flag("--place", "place", "The seed's file stem: letters, digits, '-' and '_'.", "text") },
            flag("--rules", "rules", "A store key of a rules JSON file whose fields override the baker's rules (default: the baker's own).", "file:json"),
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "The folder the pack goes into (default: world/roads/<place>).", "text"),
            DslFlag { choices: ["error", "skip", "overwrite"].iter().map(|word| word.to_string()).collect(), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("from", "Seed folder", "The store folder holding <place>.geojson, <place>.ground.json and <place>.ground.f32."),
            field("place", "Place", "The seed's file stem: letters, digits, '-' and '_'."),
            field("rules", "Rules", "A store key of a rules JSON file overriding the baker's rules. Empty: the baker's own."),
            field("folder", "Folder", "The folder the pack goes into (default: world/roads/<place>)."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["from", "place", "rules", "folder", "store", "on_conflict"].iter().map(|name| LayoutItem::Field(name.to_string())).collect(),
        failure_semantics: vec![
            NodeFailureSemantic { code: CONFIG_CODE.into(), description: "A --place that is not a plain slug, a destination that leaves the store, an --on-conflict that is not error, skip or overwrite.".into(), ..Default::default() },
            NodeFailureSemantic { code: SOURCE_CODE.into(), description: "No --from, a required seed file (<place>.geojson, .ground.json, .ground.f32) is missing, or the seed or the rules file is not readable text, or is over its ceiling.".into(), ..Default::default() },
            NodeFailureSemantic { code: SEED_CODE.into(), description: "The baker refused the seed: a ground header or grid that does not fit, road lines it cannot read, or rules it cannot take.".into(), ..Default::default() },
            NodeFailureSemantic { code: CODE.into(), description: "The bake task failed, or a store write failed.".into(), retryable: true, ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("A road pack from a seed folder", "world.roads.bake --from world/seed --place site-a --folder world/packs/roads/site-a")
                .input(json!({}))
                .output(json!({ "roads": { "folder": "world/packs/roads/site-a", "count": 12, "index": index, "items": ["…a FileRef per file"], "report": { "place": "site-a", "streets": 5, "tiles": 4, "bytes": 184000 } } }))
                .note("After `trigger.manual`. A page reads it with `bakedRoads({ url: \"<served folder>/index.json\" })`; raise `--timeout` for a big place."),
        ],
        ..Default::default()
    }
}

// ── Flags ──────────────────────────────────────────────────────────────────

/// `--place`: a plain slug, so it is a file stem and a folder name and nothing else.
fn place_slug(raw: &str) -> Result<&str, PipelineError> {
    let place = raw.trim();
    if place.is_empty() {
        return Err(PipelineError::new(CONFIG_CODE, "--place is required: the seed's file stem, e.g. site-a"));
    }
    if place.len() > MAX_PLACE_LEN || !place.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return Err(PipelineError::new(
            CONFIG_CODE,
            format!("--place '{place}' must be a plain slug of at most {MAX_PLACE_LEN} letters, digits, '-' and '_'"),
        ));
    }
    Ok(place)
}

/// `--from`: a store folder, or the answer of a node that wrote one (`{ folder }`).
fn seed_folder(from: &Value) -> Result<String, PipelineError> {
    let text = match from {
        Value::Null => "",
        Value::String(text) => text.as_str(),
        Value::Object(map) => map.get("folder").and_then(Value::as_str).ok_or_else(|| {
            PipelineError::new(SOURCE_CODE, "--from is not a folder: give a store folder, or the answer of a node that wrote one ({ folder })")
        })?,
        _ => return Err(PipelineError::new(SOURCE_CODE, "--from is not a folder: give a store folder")),
    };
    let text = text.trim().trim_end_matches('/');
    if text.is_empty() {
        return Err(PipelineError::new(SOURCE_CODE, "--from is required: the store folder holding the seed files"));
    }
    target_key(Some(text), "", "", SOURCE_CODE)
}

/// `--folder`: where the pack goes; the default is under `world/roads`.
fn destination(folder: &str, place: &str) -> Result<String, PipelineError> {
    let folder = folder.trim().trim_end_matches('/');
    let folder = if folder.is_empty() { format!("{DEFAULT_FOLDER_ROOT}/{place}") } else { folder.to_string() };
    target_key(Some(&folder), "", "", CONFIG_CODE)
}

// ── The seed ───────────────────────────────────────────────────────────────

struct Seed {
    roads: String,
    ground_json: String,
    ground_f32: Vec<u8>,
    paths: Option<String>,
    reserve: Option<String>,
    water: Option<String>,
}

/// One seed file's bytes through the store's capped read; `None` when it is
/// not there and not required. The sum of what is read stays under
/// [`MAX_SEED_BYTES`], checked before each read.
fn read_seed_file(store: &NodeStore, key: &str, required: bool, used: &mut u64) -> Result<Option<Vec<u8>>, PipelineError> {
    match store.fs.head(key) {
        Ok(stat) if stat.kind == ZebFsEntryKind::Object => {
            *used += stat.size;
            if *used > MAX_SEED_BYTES {
                return Err(PipelineError::new(SOURCE_CODE, format!("the seed is over the {MAX_SEED_BYTES} bytes a bake reads: '{key}' takes it past")));
            }
            store.read_capped(key, SOURCE_CODE).map(Some)
        }
        Ok(_) => Err(PipelineError::new(SOURCE_CODE, format!("'{key}' is a folder; the seed file is a file"))),
        Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
            if required {
                Err(PipelineError::new(SOURCE_CODE, format!("--from: the seed file '{key}' is missing")))
            } else {
                Ok(None)
            }
        }
        Err(err) => Err(PipelineError::new(SOURCE_CODE, format!("'{key}': {err}"))),
    }
}

fn utf8(bytes: Vec<u8>, key: &str) -> Result<String, PipelineError> {
    String::from_utf8(bytes).map_err(|_| PipelineError::new(SOURCE_CODE, format!("'{key}' is not UTF-8 text")))
}

fn read_seed(store: &NodeStore, folder: &str, place: &str) -> Result<Seed, PipelineError> {
    let mut used = 0u64;
    let mut files: Vec<Option<Vec<u8>>> = Vec::with_capacity(SEED_FILES.len());
    for (suffix, required) in SEED_FILES {
        let key = format!("{folder}/{place}{suffix}");
        files.push(read_seed_file(store, &key, *required, &mut used)?);
    }
    let key = |at: usize| format!("{folder}/{place}{}", SEED_FILES[at].0);
    let mut take = |at: usize| files[at].take();
    let text = |bytes: Option<Vec<u8>>, at: usize| bytes.map(|b| utf8(b, &key(at))).transpose();
    let roads = text(take(0), 0)?.unwrap_or_default();
    let ground_json = text(take(1), 1)?.unwrap_or_default();
    let ground_f32 = take(2).unwrap_or_default();
    let paths = text(take(3), 3)?;
    let reserve = text(take(4), 4)?;
    let water = text(take(5), 5)?;
    check_ground(&ground_json, ground_f32.len(), &key(1))?;
    Ok(Seed { roads, ground_json, ground_f32, paths, reserve, water })
}

/// The baker trusts the ground header: a grid of no cells, or one that reaches
/// across the planet, would loop over tiles for ever on a thread the node
/// timeout cannot stop. The header is checked here, before any thread starts.
fn check_ground(header: &str, grid_bytes: usize, key: &str) -> Result<(), PipelineError> {
    let bad = |what: String| PipelineError::new(SEED_CODE, format!("'{key}': {what}"));
    let doc: Value = serde_json::from_str(header).map_err(|err| bad(format!("not JSON: {err}")))?;
    let cells = |name: &str| doc.get(name).and_then(Value::as_u64).filter(|n| *n >= 2).ok_or_else(|| bad(format!("'{name}' must be a whole number of at least 2")));
    let (nx, nz) = (cells("nx")?, cells("nz")?);
    let step = doc.get("step").map_or(Some(4.0), Value::as_f64).filter(|s| s.is_finite() && *s > 0.0).ok_or_else(|| bad("'step' must be a number above 0".to_string()))?;
    if (nx.max(nz) - 1) as f64 * step > MAX_EXTENT_M {
        return Err(bad(format!("the grid reaches {:.0} m, over the {MAX_EXTENT_M} m a bake covers; bake the place in parts", (nx.max(nz) - 1) as f64 * step)));
    }
    if nx.saturating_mul(nz).saturating_mul(4) > grid_bytes as u64 {
        return Err(bad(format!("the grid is {nx}×{nz} cells (4 bytes each) but the .ground.f32 holds {grid_bytes} bytes")));
    }
    Ok(())
}

// ── Node ───────────────────────────────────────────────────────────────────

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        Ok(Self { config, platform })
    }

    /// `--rules` read through the store, with the baker's own override reader.
    fn rules(&self, owner: &str, project: &str) -> Result<Rules, PipelineError> {
        let named = match &self.config.rules {
            Value::Null => false,
            Value::String(text) => !text.trim().is_empty(),
            _ => true,
        };
        if !named {
            return Ok(Rules::default());
        }
        let (rules_store, key) = open_from(&self.platform, owner, project, &self.config.rules, self.config.store.as_deref(), "--rules", SOURCE_CODE)?;
        let text = utf8(rules_store.read_capped(&key, SOURCE_CODE)?, &key)?;
        Rules::from_json(Some(&text)).map_err(|message| PipelineError::new(SEED_CODE, format!("--rules '{key}': {message}")))
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;

        // Every flag is read before a byte is.
        let place = place_slug(&self.config.place)?.to_string();
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        let seed_dir = seed_folder(&self.config.from)?;
        let folder = destination(&self.config.folder, &place)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;

        // A folder that is not empty is decided before the bake is paid for.
        let index_key = format!("{folder}/index.json");
        if !on_conflict.allows_tree(&store.fs, &folder, CODE)? {
            let index = if store.fs.head(&index_key).is_ok() { store.stored_ref(&index_key, NODE_KIND, "generated", CODE)? } else { Value::Null };
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: with_answer(input.payload, json!({ "roads": { "folder": folder, "items": [], "count": 0, "index": index, "report": Value::Null } })),
                trace: vec![format!("node_kind={NODE_KIND} place={place} out={folder} skipped=true")],
            });
        }

        let rules = self.rules(owner, project)?;
        let seed = read_seed(&store, &seed_dir, &place)?;

        // The baker is a long computation on plain data: a blocking thread, one core.
        let baked = {
            let place = place.clone();
            tokio::task::spawn_blocking(move || {
                let start = Instant::now();
                let files = SeedFiles {
                    place: &place,
                    roads: &seed.roads,
                    ground_json: &seed.ground_json,
                    ground_f32: &seed.ground_f32,
                    paths: seed.paths.as_deref(),
                    reserve: seed.reserve.as_deref(),
                    water: seed.water.as_deref(),
                };
                aslnut_baker::bake(&files, &rules, &[], &|| start.elapsed().as_secs_f64())
            })
            .await
            .map_err(|err| PipelineError::new(CODE, format!("the bake task failed: {err}")))?
            .map_err(|message| PipelineError::new(SEED_CODE, format!("the baker refused the seed: {message}")))?
        };

        // The plain `.json` files only; the index goes last.
        let mut files: Vec<&(String, Vec<u8>)> = baked.files.files.iter().filter(|(rel, _)| !rel.ends_with(".json.gz")).collect();
        files.sort_by_key(|(rel, _)| rel == "index.json");
        let total: u64 = files.iter().map(|(_, bytes)| bytes.len() as u64).sum();
        if files.len() > MAX_SCRATCH_FILES || total > MAX_SCRATCH_BYTES {
            return Err(PipelineError::new(
                CODE,
                format!("the pack is {} files and {total} bytes, over the {MAX_SCRATCH_FILES} files and {MAX_SCRATCH_BYTES} bytes a tree writes; bake the place in parts", files.len()),
            ));
        }
        let mut items = Vec::with_capacity(files.len());
        let mut index = Value::Null;
        for (rel, bytes) in files {
            let key = format!("{folder}/{rel}");
            store.fs.put(&key, bytes).map_err(|err| PipelineError::new(CODE, format!("write {key}: {err}")))?;
            let filename = rel.rsplit('/').next().unwrap_or(rel);
            let file = store.file_ref(&key, filename, "application/json", bytes, NODE_KIND, "generated");
            if key == index_key {
                index = file.clone();
            }
            items.push(file);
        }
        let count = items.len();
        let seconds: f64 = baked.timings.iter().map(|(_, seconds)| seconds).sum();
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(input.payload, json!({ "roads": { "folder": folder, "items": items, "count": count, "index": index, "report": baked.summary } })),
            trace: vec![format!("node_kind={NODE_KIND} place={place} out={folder} files={count} tiles={} bytes={total} bake_s={seconds:.2}", baked.files.tiles)],
        })
    }
}
