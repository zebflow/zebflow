//! Runtime host for installed WASM nodes.
//!
//! Package discovery and node contracts live in
//! `src/platform/services/node_registry.rs`.  This module only executes an
//! already-installed package: resolve its module path, load it through Wasmtime,
//! pass the node config/input, and return a normal Zebflow node output.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use wasmtime::{Engine, Instance, Module, Store};

use crate::contracts::kinds::WASM_JSON_ABI_V1;
use crate::pipeline::model::PipelineError;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};
use crate::platform::model::{NodePackageSource, WasmModuleSpec};
use crate::platform::services::PlatformService;

const DEFAULT_ALLOC_EXPORT: &str = "zebflow_alloc";
const DEFAULT_DEALLOC_EXPORT: &str = "zebflow_dealloc";
const MAX_WASM_MODULE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_WASM_INPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_WASM_OUTPUT_BYTES: usize = 128 * 1024 * 1024;

/// Execute one installed WASM node package.
pub async fn execute_wasm_node(
    kind: String,
    config: Value,
    platform: Arc<PlatformService>,
    input: NodeExecutionInput,
) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
    let owner = input
        .metadata
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let project = input
        .metadata
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if owner.is_empty() || project.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WASM_CONTEXT",
            format!("WASM node '{kind}' requires owner/project execution metadata"),
        ));
    }

    tokio::task::spawn_blocking(move || execute_wasm_node_blocking(kind, config, platform, input))
        .await
        .map_err(|err| PipelineError::new("FW_NODE_WASM_JOIN", err.to_string()))?
}

fn execute_wasm_node_blocking(
    kind: String,
    config: Value,
    platform: Arc<PlatformService>,
    input: NodeExecutionInput,
) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
    let owner = input
        .metadata
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let project = input
        .metadata
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let installed = platform
        .node_registry
        .get_by_kind(owner, project, &kind)
        .ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_WASM_PACKAGE_NOT_FOUND",
                format!("WASM node package for '{kind}' is not installed"),
            )
        })?;
    if installed.manifest.source != NodePackageSource::Wasm {
        return Err(PipelineError::new(
            "FW_NODE_WASM_SOURCE",
            format!("node '{kind}' is installed but is not a WASM package"),
        ));
    }
    let (module_spec, export) = installed.manifest.wasm_target().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_RUNTIME",
            format!("WASM node '{kind}' has no resolvable module and export"),
        )
    })?;
    let payload = run_wasm_export(
        &kind,
        &installed.package_dir,
        module_spec,
        export,
        &config,
        &input.payload,
        &input.metadata,
    )?;

    Ok(vec![NodeExecutionOutput {
        output_pins: vec!["out".to_string()],
        payload,
        trace: vec![
            format!("node_kind={kind}"),
            format!("wasm_module={}", module_spec.path),
            format!("wasm_export={export}"),
            format!("wasm_abi={}", module_spec.abi),
        ],
    }])
}

/// Runs one export inside one declared module.
///
/// This is the single WASM entry point. Node execution and trigger lifecycle
/// hooks both call it, so a WASM node and a WASM hook behave identically.
pub fn run_wasm_export(
    label: &str,
    package_dir: &str,
    module_spec: &WasmModuleSpec,
    export: &str,
    config: &Value,
    payload: &Value,
    metadata: &Value,
) -> Result<Value, PipelineError> {
    let kind = label;
    if module_spec.abi != WASM_JSON_ABI_V1 {
        return Err(PipelineError::new(
            "FW_NODE_WASM_ABI",
            format!(
                "WASM node '{kind}' declares unsupported ABI '{}'; expected '{WASM_JSON_ABI_V1}'",
                module_spec.abi
            ),
        ));
    }
    let module_path = resolve_package_file(package_dir, &module_spec.path)?;
    let module_metadata = std::fs::metadata(&module_path).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_MODULE",
            format!(
                "WASM node '{kind}' cannot read module '{}': {err}",
                module_path.display()
            ),
        )
    })?;
    if module_metadata.len() > MAX_WASM_MODULE_BYTES {
        return Err(PipelineError::new(
            "FW_NODE_WASM_MODULE_TOO_LARGE",
            format!(
                "WASM node '{kind}' module is {} bytes; limit is {MAX_WASM_MODULE_BYTES}",
                module_metadata.len()
            ),
        ));
    }

    let engine = Engine::default();
    let module = Module::from_file(&engine, &module_path).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_COMPILE",
            format!(
                "WASM node '{kind}' failed to compile '{}': {err}",
                module_path.display()
            ),
        )
    })?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_INSTANTIATE",
            format!("WASM node '{kind}' failed to instantiate: {err}"),
        )
    })?;

    execute_json_abi(
        kind,
        module_spec,
        &mut store,
        &instance,
        export,
        config,
        payload,
        metadata,
    )
}

fn resolve_package_file(package_dir: &str, rel_path: &str) -> Result<PathBuf, PipelineError> {
    let rel = Path::new(rel_path);
    if rel.is_absolute()
        || rel
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(PipelineError::new(
            "FW_NODE_WASM_PATH",
            format!("WASM module path '{rel_path}' must be package-relative"),
        ));
    }
    Ok(Path::new(package_dir).join(rel))
}

#[allow(clippy::too_many_arguments)]
fn execute_json_abi(
    kind: &str,
    module_spec: &WasmModuleSpec,
    store: &mut Store<()>,
    instance: &Instance,
    main_export: &str,
    config: &Value,
    payload: &Value,
    metadata: &Value,
) -> Result<Value, PipelineError> {
    let request = json!({
        "config": config,
        "input": payload,
        "context": metadata,
    });
    let bytes = serde_json::to_vec(&request).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_JSON_INPUT",
            format!("WASM node '{kind}' input serialization failed: {err}"),
        )
    })?;
    if bytes.len() > MAX_WASM_INPUT_BYTES {
        return Err(PipelineError::new(
            "FW_NODE_WASM_INPUT_TOO_LARGE",
            format!(
                "WASM node '{kind}' input is {} bytes; limit is {MAX_WASM_INPUT_BYTES}",
                bytes.len()
            ),
        ));
    }

    let memory = instance.get_memory(&mut *store, "memory").ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_MEMORY",
            format!(
                "WASM node '{kind}' ABI '{}' requires exported memory",
                module_spec.abi
            ),
        )
    })?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&mut *store, DEFAULT_ALLOC_EXPORT)
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_WASM_ALLOC",
                format!("WASM node '{kind}' missing {DEFAULT_ALLOC_EXPORT}: {err}"),
            )
        })?;
    let run = instance
        .get_typed_func::<(i32, i32), i64>(&mut *store, main_export)
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_WASM_RUN",
                format!("WASM node '{kind}' missing JSON run export '{main_export}': {err}"),
            )
        })?;

    let ptr = alloc.call(&mut *store, bytes.len() as i32).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_ALLOC_CALL",
            format!("WASM node '{kind}' allocation failed: {err}"),
        )
    })? as usize;
    write_memory(kind, store, &memory, ptr, &bytes)?;
    let ret = run
        .call(&mut *store, (ptr as i32, bytes.len() as i32))
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_WASM_RUN_CALL",
                format!("WASM node '{kind}' JSON run failed: {err}"),
            )
        })?;
    let out_ptr = (ret >> 32) as u32 as usize;
    let out_len = (ret & 0xffff_ffff) as u32 as usize;
    if out_len > MAX_WASM_OUTPUT_BYTES {
        return Err(PipelineError::new(
            "FW_NODE_WASM_OUTPUT_TOO_LARGE",
            format!(
                "WASM node '{kind}' output is {out_len} bytes; limit is {MAX_WASM_OUTPUT_BYTES}"
            ),
        ));
    }
    let out = read_memory(kind, store, &memory, out_ptr, out_len)?;
    if let Ok(dealloc) =
        instance.get_typed_func::<(i32, i32), ()>(&mut *store, DEFAULT_DEALLOC_EXPORT)
    {
        let _ = dealloc.call(&mut *store, (out_ptr as i32, out_len as i32));
    }
    serde_json::from_slice(&out).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_JSON_OUTPUT",
            format!("WASM node '{kind}' returned invalid JSON: {err}"),
        )
    })
}

fn write_memory(
    kind: &str,
    store: &mut Store<()>,
    memory: &wasmtime::Memory,
    ptr: usize,
    bytes: &[u8],
) -> Result<(), PipelineError> {
    let data = memory.data_mut(store);
    let end = ptr.checked_add(bytes.len()).ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_MEMORY",
            format!("WASM node '{kind}' pointer overflow"),
        )
    })?;
    if end > data.len() {
        return Err(PipelineError::new(
            "FW_NODE_WASM_MEMORY",
            format!("WASM node '{kind}' allocation returned out-of-bounds pointer"),
        ));
    }
    data[ptr..end].copy_from_slice(bytes);
    Ok(())
}

fn read_memory(
    kind: &str,
    store: &mut Store<()>,
    memory: &wasmtime::Memory,
    ptr: usize,
    len: usize,
) -> Result<Vec<u8>, PipelineError> {
    let data = memory.data(store);
    let end = ptr.checked_add(len).ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_MEMORY",
            format!("WASM node '{kind}' pointer overflow"),
        )
    })?;
    if end > data.len() {
        return Err(PipelineError::new(
            "FW_NODE_WASM_MEMORY",
            format!("WASM node '{kind}' returned out-of-bounds output pointer"),
        ));
    }
    Ok(data[ptr..end].to_vec())
}
