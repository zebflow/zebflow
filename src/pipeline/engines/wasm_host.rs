//! Runtime host for installed `n.wasm.*` node packages.
//!
//! Package discovery and node contracts live in
//! `src/platform/services/node_registry.rs`.  This module only executes an
//! already-installed package: resolve its module path, load it through Wasmtime,
//! pass the node config/input, and return a normal Zebflow node output.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use wasmtime::{Engine, Instance, Module, Store, Val, ValType};

use crate::pipeline::model::PipelineError;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};
use crate::platform::model::{NodePackageSource, WasmNodeRuntime};
use crate::platform::services::PlatformService;

const DEFAULT_MAIN_EXPORT: &str = "zebflow_wasm_run";
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
    let runtime = installed.manifest.wasm_runtime.as_ref().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_RUNTIME",
            format!("WASM node '{kind}' has no wasm_runtime/module declaration"),
        )
    })?;
    let module_path = resolve_package_file(&installed.package_dir, &runtime.module)?;
    let metadata = std::fs::metadata(&module_path).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_MODULE",
            format!(
                "WASM node '{kind}' cannot read module '{}': {err}",
                module_path.display()
            ),
        )
    })?;
    if metadata.len() > MAX_WASM_MODULE_BYTES {
        return Err(PipelineError::new(
            "FW_NODE_WASM_MODULE_TOO_LARGE",
            format!(
                "WASM node '{kind}' module is {} bytes; limit is {MAX_WASM_MODULE_BYTES}",
                metadata.len()
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

    let main_export = runtime
        .exports
        .get("main")
        .map(String::as_str)
        .unwrap_or(DEFAULT_MAIN_EXPORT);
    let payload = if runtime.abi == "zebflow-wasm-json-v1"
        && has_json_abi(&mut store, &instance, main_export)
    {
        execute_json_abi(
            &kind,
            runtime,
            &mut store,
            &instance,
            main_export,
            &config,
            &input,
        )?
    } else {
        execute_scalar_abi(
            &kind,
            &mut store,
            &instance,
            main_export,
            &installed.manifest.definition.fields,
            &config,
            &input.payload,
        )?
    };

    Ok(vec![NodeExecutionOutput {
        output_pins: vec!["out".to_string()],
        payload,
        trace: vec![
            format!("node_kind={kind}"),
            format!("wasm_module={}", runtime.module),
            format!("wasm_abi={}", runtime.abi),
        ],
    }])
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

fn has_json_abi(store: &mut Store<()>, instance: &Instance, main_export: &str) -> bool {
    instance.get_memory(&mut *store, "memory").is_some()
        && instance
            .get_typed_func::<i32, i32>(&mut *store, DEFAULT_ALLOC_EXPORT)
            .is_ok()
        && instance
            .get_typed_func::<(i32, i32), i64>(&mut *store, main_export)
            .is_ok()
}

fn execute_json_abi(
    kind: &str,
    runtime: &WasmNodeRuntime,
    store: &mut Store<()>,
    instance: &Instance,
    main_export: &str,
    config: &Value,
    input: &NodeExecutionInput,
) -> Result<Value, PipelineError> {
    let request = json!({
        "config": config,
        "input": input.payload,
        "context": input.metadata,
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
                runtime.abi
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

fn execute_scalar_abi(
    kind: &str,
    store: &mut Store<()>,
    instance: &Instance,
    main_export: &str,
    fields: &[crate::pipeline::NodeFieldDef],
    config: &Value,
    input: &Value,
) -> Result<Value, PipelineError> {
    let func = instance.get_func(&mut *store, main_export).ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WASM_EXPORT",
            format!("WASM node '{kind}' missing main export '{main_export}'"),
        )
    })?;
    let ty = func.ty(&mut *store);
    let params: Vec<ValType> = ty.params().collect();
    let results: Vec<ValType> = ty.results().collect();
    let mut args = Vec::new();
    for (idx, param) in params.iter().enumerate() {
        let name = fields
            .get(idx)
            .map(|field| field.name.as_str())
            .unwrap_or_default();
        let value = config
            .get(name)
            .or_else(|| input.get(name))
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_WASM_ARG",
                    format!("WASM node '{kind}' scalar argument {idx} ('{name}') is missing"),
                )
            })?;
        args.push(json_value_to_wasm_val(kind, idx, name, param, value)?);
    }
    let mut out = results
        .iter()
        .map(default_wasm_val)
        .collect::<Result<Vec<_>, _>>()?;
    func.call(&mut *store, &args, &mut out).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WASM_CALL",
            format!("WASM node '{kind}' scalar export '{main_export}' failed: {err}"),
        )
    })?;
    let result_values = out.iter().map(wasm_val_to_json).collect::<Vec<_>>();
    let result = if result_values.len() == 1 {
        result_values[0].clone()
    } else {
        Value::Array(result_values)
    };
    Ok(json!({
        "ok": true,
        "wasm": {
            "export": main_export,
            "result": result
        }
    }))
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

fn json_value_to_wasm_val(
    kind: &str,
    idx: usize,
    name: &str,
    ty: &ValType,
    value: &Value,
) -> Result<Val, PipelineError> {
    match ty {
        ValType::I32 => value
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .map(Val::I32)
            .ok_or_else(|| scalar_type_error(kind, idx, name, "i32")),
        ValType::I64 => value
            .as_i64()
            .map(Val::I64)
            .ok_or_else(|| scalar_type_error(kind, idx, name, "i64")),
        ValType::F32 => value
            .as_f64()
            .map(|n| Val::F32((n as f32).to_bits()))
            .ok_or_else(|| scalar_type_error(kind, idx, name, "f32")),
        ValType::F64 => value
            .as_f64()
            .map(|n| Val::F64(n.to_bits()))
            .ok_or_else(|| scalar_type_error(kind, idx, name, "f64")),
        other => Err(PipelineError::new(
            "FW_NODE_WASM_TYPE",
            format!("WASM node '{kind}' argument {idx} ('{name}') uses unsupported type {other:?}"),
        )),
    }
}

fn default_wasm_val(ty: &ValType) -> Result<Val, PipelineError> {
    match ty {
        ValType::I32 => Ok(Val::I32(0)),
        ValType::I64 => Ok(Val::I64(0)),
        ValType::F32 => Ok(Val::F32(0)),
        ValType::F64 => Ok(Val::F64(0)),
        other => Err(PipelineError::new(
            "FW_NODE_WASM_TYPE",
            format!("unsupported WASM result type {other:?}"),
        )),
    }
}

fn wasm_val_to_json(value: &Val) -> Value {
    match value {
        Val::I32(n) => json!(*n),
        Val::I64(n) => json!(*n),
        Val::F32(bits) => json!(f32::from_bits(*bits)),
        Val::F64(bits) => json!(f64::from_bits(*bits)),
        other => json!(format!("{other:?}")),
    }
}

fn scalar_type_error(kind: &str, idx: usize, name: &str, expected: &str) -> PipelineError {
    PipelineError::new(
        "FW_NODE_WASM_ARG_TYPE",
        format!("WASM node '{kind}' argument {idx} ('{name}') must be {expected}"),
    )
}
