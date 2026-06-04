//! Deno sandbox language engine implementation.
//!
//! This file owns:
//!
//! - compile-time policy merge and instrumentation
//! - runtime artifact serialization/deserialization
//! - `LanguageEngine` integration

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::interface::LanguageEngine;
use crate::language::model::{
    CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput, LanguageError,
    ModuleSource, ProgramIr,
};

use super::config::{DenoSandboxConfig, DenoSandboxConfigPatch, apply_patch, normalize_limits};
use super::instrument::{forbid_patterns, inject_loop_guards};
use super::runner::run_compiled_script;

const TOOL_INIT: &str = include_str!("../../../language/runtime/tool_init.js");

/// Serialized artifact produced by `DenoSandboxEngine::compile_script`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledDenoSandboxScript {
    /// Stable hash id of the transformed module + critical limits.
    pub script_id: String,
    /// Original source submitted by caller.
    pub original_source: String,
    /// Wrapped module source (kept for backward compat with stored artifacts).
    pub module_source: String,
    /// Async function expression for pool execution (no `export default`).
    /// Format: `async function(input, n, ctx) { <body> }`
    pub fn_source: String,
    /// Resolved, normalized runtime policy.
    pub resolved_config: DenoSandboxConfig,
}

/// Sandboxed Deno language engine with layered policy patches.
///
/// Layer merge order:
///
/// 1. strict defaults
/// 2. platform patch
/// 3. project patch
/// 4. optional run patch
#[derive(Clone, Debug, Default)]
pub struct DenoSandboxEngine {
    /// Global platform patch.
    pub platform_patch: DenoSandboxConfigPatch,
    /// Per-project patch.
    pub project_patch: DenoSandboxConfigPatch,
}

impl DenoSandboxEngine {
    /// Creates a new engine with explicit platform and project patches.
    pub fn new(
        platform_patch: DenoSandboxConfigPatch,
        project_patch: DenoSandboxConfigPatch,
    ) -> Self {
        Self {
            platform_patch,
            project_patch,
        }
    }

    /// Compiles plain JS source into a sandbox-ready artifact.
    pub fn compile_script(
        &self,
        source: &str,
        run_patch: Option<&DenoSandboxConfigPatch>,
    ) -> Result<CompiledDenoSandboxScript, LanguageError> {
        let mut cfg = DenoSandboxConfig::default();
        apply_patch(&mut cfg, &self.platform_patch);
        apply_patch(&mut cfg, &self.project_patch);
        if let Some(patch) = run_patch {
            apply_patch(&mut cfg, patch);
        }
        normalize_limits(&mut cfg);

        if source.len() > cfg.max_source_bytes {
            return Err(LanguageError::new(
                "LANG_DENO_SOURCE_TOO_LARGE",
                format!(
                    "source too large ({} > {} bytes)",
                    source.len(),
                    cfg.max_source_bytes
                ),
            ));
        }

        forbid_patterns(source, &cfg).map_err(|e| {
            LanguageError::new("LANG_DENO_POLICY", format!("policy violation: {e}"))
        })?;

        let body = if cfg.danger_zone.disable_loop_guards {
            source.to_string()
        } else {
            inject_loop_guards(source)
        };

        let fn_source = format!("async function(input, n, ctx) {{\n{body}\n}}");
        let module_source = format!("{TOOL_INIT}\nexport default {fn_source}\n");

        let mut hasher = DefaultHasher::new();
        module_source.hash(&mut hasher);
        cfg.timeout_ms.hash(&mut hasher);
        cfg.max_ops.hash(&mut hasher);
        let script_id = format!("tj_{:x}", hasher.finish());

        Ok(CompiledDenoSandboxScript {
            script_id,
            original_source: source.to_string(),
            module_source,
            fn_source,
            resolved_config: cfg,
        })
    }

    /// Runs an already compiled script with JSON input and optional execution context.
    pub fn run_compiled(
        &self,
        compiled: &CompiledDenoSandboxScript,
        input: &Value,
        ctx: Value,
    ) -> Result<Value, LanguageError> {
        run_compiled_script(compiled, input, ctx).map_err(|e| {
            LanguageError::new(
                "LANG_DENO_RUN",
                format!("sandbox execution failed for '{}': {e}", compiled.script_id),
            )
        })
    }

    /// Convenience path for compile + run in one call.
    pub fn run_script(
        &self,
        source: &str,
        input: &Value,
        run_patch: Option<&DenoSandboxConfigPatch>,
    ) -> Result<Value, LanguageError> {
        let compiled = self.compile_script(source, run_patch)?;
        self.run_compiled(&compiled, input, Value::Null)
    }
}

impl LanguageEngine for DenoSandboxEngine {
    fn id(&self) -> &'static str {
        "language.deno_sandbox"
    }

    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError> {
        Ok(ProgramIr {
            source_id: module.id.clone(),
            kind: module.kind,
            body: json!({
                "source": module.code,
                "source_kind": format!("{:?}", module.kind),
            }),
        })
    }

    fn compile(
        &self,
        ir: &ProgramIr,
        options: &CompileOptions,
    ) -> Result<CompiledProgram, LanguageError> {
        let source = ir
            .body
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LanguageError::new(
                    "LANG_DENO_COMPILE_INPUT",
                    format!("missing source text in ir '{}'", ir.source_id),
                )
            })?;

        let compiled = self.compile_script(source, None)?;
        let artifact = serde_json::to_vec(&compiled).map_err(|e| {
            LanguageError::new(
                "LANG_DENO_COMPILE_ENCODE",
                format!(
                    "failed to serialize compiled artifact '{}': {e}",
                    ir.source_id
                ),
            )
        })?;

        Ok(CompiledProgram {
            engine_id: self.id().to_string(),
            source_id: ir.source_id.clone(),
            artifact,
            metadata: json!({
                "target": options.target,
                "optimize_level": options.optimize_level,
                "emit_trace_hints": options.emit_trace_hints,
            }),
        })
    }

    fn run(
        &self,
        compiled: &CompiledProgram,
        input: Value,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutput, LanguageError> {
        let mut script: CompiledDenoSandboxScript = serde_json::from_slice(&compiled.artifact)
            .map_err(|e| {
                LanguageError::new(
                    "LANG_DENO_ARTIFACT_DECODE",
                    format!(
                        "failed to decode compiled artifact '{}': {e}",
                        compiled.source_id
                    ),
                )
            })?;

        if let Some(run_patch) = extract_run_patch(&ctx.metadata)? {
            script = self.compile_script(&script.original_source, Some(&run_patch))?;
        }

        let ctx_json = json!({
            "pipeline": ctx.pipeline,
            "request_id": ctx.request_id,
            "trigger": ctx.trigger,
            "nodes": ctx.metadata.get("nodes").cloned().unwrap_or_else(|| json!({})),
            "placeholder": ctx.metadata.get("placeholder").cloned().unwrap_or_else(|| json!({})),
        });
        let value = self.run_compiled(&script, &input, ctx_json)?;

        Ok(ExecutionOutput {
            value,
            trace: vec![
                format!("engine={}", self.id()),
                format!("script_id={}", script.script_id),
                format!("project={}", ctx.project),
                format!("pipeline={}", ctx.pipeline),
                format!("request_id={}", ctx.request_id),
            ],
        })
    }
}

/// Extracts an optional per-run sandbox patch from execution metadata.
///
/// Accepted keys:
///
/// - `languageRunPatch`
/// - `denoSandboxRunPatch` (backward compatibility)
fn extract_run_patch(metadata: &Value) -> Result<Option<DenoSandboxConfigPatch>, LanguageError> {
    let raw = metadata
        .get("languageRunPatch")
        .or_else(|| metadata.get("denoSandboxRunPatch"));
    let Some(raw) = raw else {
        return Ok(None);
    };
    let patch = serde_json::from_value(raw.clone()).map_err(|e| {
        LanguageError::new(
            "LANG_DENO_RUN_PATCH",
            format!("invalid metadata.languageRunPatch|denoSandboxRunPatch: {e}"),
        )
    })?;
    Ok(Some(patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_tool_script(source: &str) -> Value {
        DenoSandboxEngine::default()
            .run_script(source, &json!({}), None)
            .expect("tool script should succeed")
    }

    fn approx_eq(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    #[test]
    fn tool_geo_distance_returns_km_and_supports_legacy_args() {
        let out = run_tool_script(
            r#"
return {
  array_distance: Tool.geo.distance([0, 0], [0, 1]),
  legacy_distance: Tool.geo.distance(0, 0, 1, 0),
};
"#,
        );

        let array_distance = out["array_distance"].as_f64().unwrap();
        let legacy_distance = out["legacy_distance"].as_f64().unwrap();

        approx_eq(array_distance, 111.195, 0.5);
        approx_eq(legacy_distance, 111.195, 0.5);
    }

    #[test]
    fn tool_geo_point_in_polygon_supports_geojson_holes() {
        let out = run_tool_script(
            r#"
const polygon = {
  type: "Polygon",
  coordinates: [
    [[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]],
    [[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]
  ]
};

return {
  inside_outer: Tool.geo.pointInPolygon([0.5, 0.5], polygon),
  inside_hole: Tool.geo.pointInPolygon([2, 2], polygon),
  outside: Tool.geo.pointInPolygon([6, 6], polygon),
};
"#,
        );

        assert_eq!(out["inside_outer"].as_bool(), Some(true));
        assert_eq!(out["inside_hole"].as_bool(), Some(false));
        assert_eq!(out["outside"].as_bool(), Some(false));
    }

    #[test]
    fn tool_geo_centroid_supports_polygon_and_multipolygon() {
        let out = run_tool_script(
            r#"
const polygon = {
  type: "Polygon",
  coordinates: [
    [[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]]
  ]
};

const multiPolygon = {
  type: "MultiPolygon",
  coordinates: [
    [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
    [[[3, 0], [4, 0], [4, 1], [3, 1], [3, 0]]]
  ]
};

return {
  polygon: Tool.geo.centroid(polygon),
  multi_polygon: Tool.geo.centroid(multiPolygon),
};
"#,
        );

        let polygon = out["polygon"].as_array().unwrap();
        let multi_polygon = out["multi_polygon"].as_array().unwrap();

        approx_eq(polygon[0].as_f64().unwrap(), 2.0, 1e-6);
        approx_eq(polygon[1].as_f64().unwrap(), 2.0, 1e-6);
        approx_eq(multi_polygon[0].as_f64().unwrap(), 2.0, 1e-6);
        approx_eq(multi_polygon[1].as_f64().unwrap(), 0.5, 1e-6);
    }

    #[test]
    fn tool_geo_nearest_point_returns_index_and_distance() {
        let out = run_tool_script(
            r#"
return Tool.geo.nearestPoint(
  [0, 0],
  [
    [0, 2],
    [0, 0.5],
    [3, 3]
  ]
);
"#,
        );

        assert_eq!(out["index"].as_i64(), Some(1));
        approx_eq(out["distance"].as_f64().unwrap(), 55.597, 0.5);
        let point = out["point"].as_array().unwrap();
        approx_eq(point[0].as_f64().unwrap(), 0.0, 1e-6);
        approx_eq(point[1].as_f64().unwrap(), 0.5, 1e-6);
    }

    #[test]
    fn tool_geo_supports_wkt_parsing_and_heading() {
        let out = run_tool_script(
            r#"
const coords = Tool.geo.parseWktLineString("LINESTRING (145.1 -37.9, 145.2 -37.8, 145.3 -37.85)");
return {
  coords,
  heading: Tool.geo.heading(coords[0], coords[1]),
  inside: Tool.geo.booleanPointInPolygon([0.5, 0.5], {
    type: "Polygon",
    coordinates: [
      [[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]
    ]
  }),
};
"#,
        );

        let coords = out["coords"].as_array().unwrap();
        assert_eq!(coords.len(), 3);
        approx_eq(coords[0][0].as_f64().unwrap(), 145.1, 1e-6);
        approx_eq(coords[0][1].as_f64().unwrap(), -37.9, 1e-6);
        assert_eq!(out["inside"].as_bool(), Some(true));
        approx_eq(out["heading"].as_f64().unwrap(), 45.0, 10.0);
    }

    #[test]
    fn tool_geo_supports_route_progress_interpolation_and_bearing() {
        let out = run_tool_script(
            r#"
const route = [
  [145.1, -37.9],
  [145.2, -37.8],
  [145.3, -37.85]
];
const progress = Tool.geo.routeProgress(route);
return {
  total: progress.totalDistance,
  distances: progress.distances,
  midpoint: Tool.geo.interpolateRoute(route, 0.5, progress),
  bearing: Tool.geo.bearing(route[0], route[1]),
};
"#,
        );

        assert!(out["total"].as_f64().unwrap() > 0.0);
        let distances = out["distances"].as_array().unwrap();
        assert_eq!(distances.len(), 3);
        let midpoint = out["midpoint"].as_array().unwrap();
        assert_eq!(midpoint.len(), 2);
        assert!(midpoint[0].as_f64().unwrap() > 145.1);
        assert!(midpoint[0].as_f64().unwrap() < 145.3);
        let bearing = out["bearing"].as_f64().unwrap();
        assert!(bearing >= 0.0);
        assert!(bearing < 360.0);
    }

    #[test]
    fn tool_csv_parses_and_stringifies_rows() {
        let out = run_tool_script(
            r#"
const rows = Tool.csv.parse("id,name\n1,Alice\n2,Bob");
return {
  rows,
  csv: Tool.csv.stringify(rows, { columns: ["id", "name"] }),
};
"#,
        );

        let rows = out["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"].as_str(), Some("1"));
        assert_eq!(rows[1]["name"].as_str(), Some("Bob"));
        assert!(out["csv"].as_str().unwrap().contains("Alice"));
    }
}
