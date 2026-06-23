//! Prototype: Visual Rust execution model.
//!
//! This file is a deliberately small reference implementation for Zebflow's
//! "visual Rust backend" direction. It keeps the node graph idea, but payloads
//! follow Rust ownership:
//!
//! - a node receives one [`VisualPayload`] by value;
//! - the node returns the next [`VisualPayload`] by value;
//! - trace stores summaries, never full payload copies;
//! - nested function calls execute subgraphs inline and move the payload through;
//! - large data is typed (`VectorBatch`) rather than universal JSON.
//!
//! The production engine lives in [`crate::pipeline::engines`]. This prototype
//! exists to define the target semantics and provide benchmarks before the main
//! runtime is refactored toward this model.

use std::fmt;
use std::time::{Duration, Instant};

/// Payload flowing between visual nodes.
///
/// This enum intentionally does **not** implement [`Clone`]. A graph branch or
/// a tracing system that wants to duplicate large data must introduce an
/// explicit sharing or handle policy instead of accidentally cloning the value.
#[derive(Debug)]
pub enum VisualPayload {
    Unit,
    Json(serde_json::Value),
    VectorBatch(VectorBatch),
    FileRef(FileRef),
    TableRef(TableRef),
}

impl VisualPayload {
    pub fn summary(&self) -> PayloadSummary {
        match self {
            Self::Unit => PayloadSummary {
                kind: "unit",
                rows: 0,
                dims: 0,
                bytes: 0,
            },
            Self::Json(value) => PayloadSummary {
                kind: "json",
                rows: value.as_array().map_or(1, Vec::len),
                dims: 0,
                bytes: json_size_hint(value),
            },
            Self::VectorBatch(batch) => PayloadSummary {
                kind: "vector_batch",
                rows: batch.rows(),
                dims: batch.dims,
                bytes: batch.byte_len(),
            },
            Self::FileRef(file) => PayloadSummary {
                kind: "file_ref",
                rows: 0,
                dims: 0,
                bytes: file.size_bytes,
            },
            Self::TableRef(table) => PayloadSummary {
                kind: "table_ref",
                rows: table.rows.unwrap_or_default(),
                dims: 0,
                bytes: table.size_bytes.unwrap_or_default(),
            },
        }
    }
}

/// Typed vector payload.
///
/// Vectors are stored flat to avoid `Vec<Vec<f32>>` allocation overhead. For
/// `rows = 100` and `dims = 4096`, `values.len() == 409_600`.
#[derive(Debug)]
pub struct VectorBatch {
    pub keys: Vec<String>,
    pub labels: Vec<String>,
    pub values: Vec<f32>,
    pub dims: usize,
}

impl VectorBatch {
    pub fn new(keys: Vec<String>, labels: Vec<String>, values: Vec<f32>, dims: usize) -> Self {
        debug_assert_eq!(keys.len(), labels.len());
        debug_assert_eq!(keys.len().saturating_mul(dims), values.len());
        Self {
            keys,
            labels,
            values,
            dims,
        }
    }

    pub fn rows(&self) -> usize {
        self.keys.len()
    }

    pub fn byte_len(&self) -> usize {
        self.values.len() * std::mem::size_of::<f32>()
    }
}

#[derive(Debug)]
pub struct FileRef {
    pub path: String,
    pub size_bytes: usize,
}

#[derive(Debug)]
pub struct TableRef {
    pub path: String,
    pub rows: Option<usize>,
    pub size_bytes: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadSummary {
    pub kind: &'static str,
    pub rows: usize,
    pub dims: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone)]
pub struct VisualTraceEntry {
    pub node: String,
    pub input: PayloadSummary,
    pub output: PayloadSummary,
    pub duration: Duration,
}

#[derive(Debug, Default)]
pub struct VisualTrace {
    entries: Vec<VisualTraceEntry>,
}

impl VisualTrace {
    pub fn push(&mut self, entry: VisualTraceEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[VisualTraceEntry] {
        &self.entries
    }
}

#[derive(Debug, Default)]
pub struct VisualCtx {
    pub trace: VisualTrace,
    pub store: VectorStore,
}

pub trait VisualNode: fmt::Debug {
    fn name(&self) -> &str;
    fn run(&self, input: VisualPayload, ctx: &mut VisualCtx) -> Result<VisualPayload, VisualError>;
}

#[derive(Debug, Default)]
pub struct VisualGraph {
    nodes: Vec<Box<dyn VisualNode>>,
}

impl VisualGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<N>(&mut self, node: N)
    where
        N: VisualNode + 'static,
    {
        self.nodes.push(Box::new(node));
    }

    pub fn run(
        &self,
        mut payload: VisualPayload,
        ctx: &mut VisualCtx,
    ) -> Result<VisualPayload, VisualError> {
        for node in &self.nodes {
            let input_summary = payload.summary();
            let started = Instant::now();
            let output = node.run(payload, ctx)?;
            let output_summary = output.summary();
            ctx.trace.push(VisualTraceEntry {
                node: node.name().to_string(),
                input: input_summary,
                output: output_summary,
                duration: started.elapsed(),
            });
            payload = output;
        }
        Ok(payload)
    }
}

#[derive(Debug)]
pub struct CallGraphNode {
    name: String,
    graph: VisualGraph,
}

impl CallGraphNode {
    pub fn new(name: impl Into<String>, graph: VisualGraph) -> Self {
        Self {
            name: name.into(),
            graph,
        }
    }
}

impl VisualNode for CallGraphNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, input: VisualPayload, ctx: &mut VisualCtx) -> Result<VisualPayload, VisualError> {
        self.graph.run(input, ctx)
    }
}

#[derive(Debug)]
pub struct IdentityNode {
    name: String,
}

impl IdentityNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl VisualNode for IdentityNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(
        &self,
        input: VisualPayload,
        _ctx: &mut VisualCtx,
    ) -> Result<VisualPayload, VisualError> {
        Ok(input)
    }
}

#[derive(Debug)]
pub struct PutVectorsNode {
    name: String,
    mode: VectorStoreMode,
}

impl PutVectorsNode {
    pub fn new(name: impl Into<String>, mode: VectorStoreMode) -> Self {
        Self {
            name: name.into(),
            mode,
        }
    }
}

impl VisualNode for PutVectorsNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, input: VisualPayload, ctx: &mut VisualCtx) -> Result<VisualPayload, VisualError> {
        let VisualPayload::VectorBatch(batch) = input else {
            return Err(VisualError::WrongPayload {
                node: self.name.clone(),
                expected: "VectorBatch",
            });
        };
        let rows = batch.rows();
        let dims = batch.dims;
        ctx.store.write(batch, self.mode);
        Ok(VisualPayload::Json(serde_json::json!({
            "ok": true,
            "rows": rows,
            "dims": dims
        })))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorStoreMode {
    /// Simulates an external/vector-service write. The payload is consumed, then
    /// dropped. The process should not keep the vectors resident.
    CountOnly,
    /// Simulates an embedded in-process vector store. Vectors remain resident in
    /// the process after writes complete.
    Resident,
}

#[derive(Debug, Default)]
pub struct VectorStore {
    pub rows: usize,
    pub dims: usize,
    pub resident_values: Vec<f32>,
}

impl VectorStore {
    fn write(&mut self, batch: VectorBatch, mode: VectorStoreMode) {
        self.rows += batch.rows();
        self.dims = batch.dims;
        match mode {
            VectorStoreMode::CountOnly => {}
            VectorStoreMode::Resident => self.resident_values.extend(batch.values),
        }
    }
}

#[derive(Debug)]
pub enum VisualError {
    WrongPayload {
        node: String,
        expected: &'static str,
    },
}

impl fmt::Display for VisualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPayload { node, expected } => {
                write!(f, "node '{node}' expected payload {expected}")
            }
        }
    }
}

impl std::error::Error for VisualError {}

pub fn nested_vector_writer_graph(mode: VectorStoreMode) -> VisualGraph {
    let mut writer = VisualGraph::new();
    writer.push(PutVectorsNode::new("writer.put_vectors", mode));

    let mut middle = VisualGraph::new();
    middle.push(CallGraphNode::new("middle.call_writer", writer));

    let mut outer = VisualGraph::new();
    outer.push(CallGraphNode::new("outer.call_middle", middle));
    outer
}

pub fn make_vector_batch(start: usize, rows: usize, dims: usize) -> VectorBatch {
    let mut keys = Vec::with_capacity(rows);
    let mut labels = Vec::with_capacity(rows);
    let mut values = Vec::with_capacity(rows * dims);
    for row in 0..rows {
        let seed = start + row;
        keys.push(format!("v{seed:08}"));
        labels.push(format!("label_{seed}"));
        for dim in 0..dims {
            values.push(vector_value(seed, dim));
        }
    }
    VectorBatch::new(keys, labels, values, dims)
}

fn vector_value(seed: usize, dim: usize) -> f32 {
    let mixed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(dim.wrapping_mul(1_442_695_040_888_963_407));
    let x = (mixed >> 33) as u32;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn json_size_hint(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => 8,
        serde_json::Value::String(s) => s.len(),
        serde_json::Value::Array(items) => items.iter().map(json_size_hint).sum(),
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, value)| key.len() + json_size_hint(value))
            .sum(),
    }
}

#[derive(Debug, Clone)]
pub struct VisualBenchmarkConfig {
    pub total_rows: usize,
    pub batch_size: usize,
    pub dims: usize,
    pub mode: VectorStoreMode,
}

#[derive(Debug, Clone)]
pub struct VisualBenchmarkReport {
    pub config: VisualBenchmarkConfig,
    pub rows: usize,
    pub batches: usize,
    pub elapsed: Duration,
    pub rows_per_second: f64,
    pub trace_entries: usize,
    pub observed_payload_bytes: usize,
    pub peak_rss_mb: Option<f64>,
    pub final_rss_mb: Option<f64>,
}

pub fn run_vector_ingest_benchmark(
    config: VisualBenchmarkConfig,
    rss_sampler: impl Fn() -> Option<f64>,
) -> Result<VisualBenchmarkReport, VisualError> {
    let graph = nested_vector_writer_graph(config.mode);
    let mut ctx = VisualCtx::default();
    let started = Instant::now();
    let mut peak_rss_mb = rss_sampler();
    let mut rows_written = 0;
    let mut batches = 0;

    for start in (0..config.total_rows).step_by(config.batch_size) {
        let rows = config.batch_size.min(config.total_rows - start);
        let batch = make_vector_batch(start, rows, config.dims);
        let output = graph.run(VisualPayload::VectorBatch(batch), &mut ctx)?;
        drop(output);
        rows_written += rows;
        batches += 1;
        if let Some(rss) = rss_sampler() {
            peak_rss_mb = Some(peak_rss_mb.map_or(rss, |peak| peak.max(rss)));
        }
    }

    let elapsed = started.elapsed();
    let observed_payload_bytes = ctx
        .trace
        .entries()
        .iter()
        .map(|entry| entry.input.bytes + entry.output.bytes)
        .sum();

    Ok(VisualBenchmarkReport {
        config,
        rows: rows_written,
        batches,
        elapsed,
        rows_per_second: rows_written as f64 / elapsed.as_secs_f64(),
        trace_entries: ctx.trace.entries().len(),
        observed_payload_bytes,
        peak_rss_mb,
        final_rss_mb: rss_sampler(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn visual_rust_nested_function_smoke() {
        let graph = nested_vector_writer_graph(VectorStoreMode::CountOnly);
        let mut ctx = VisualCtx::default();
        let batch = make_vector_batch(0, 4, 8);
        let output = graph
            .run(VisualPayload::VectorBatch(batch), &mut ctx)
            .expect("prototype graph should run");

        assert!(matches!(output, VisualPayload::Json(_)));
        assert_eq!(ctx.store.rows, 4);
        assert_eq!(ctx.store.dims, 8);
        assert_eq!(ctx.trace.entries().len(), 3);
        assert_eq!(ctx.trace.entries()[0].input.kind, "vector_batch");
    }

    #[test]
    #[ignore = "benchmark: writes report to /tmp and allocates large vector batches"]
    fn visual_rust_10000_4096_benchmark() {
        let count_only = run_vector_ingest_benchmark(
            VisualBenchmarkConfig {
                total_rows: 10_000,
                batch_size: 100,
                dims: 4096,
                mode: VectorStoreMode::CountOnly,
            },
            current_rss_mb,
        )
        .expect("count-only visual benchmark");
        let resident = run_vector_ingest_benchmark(
            VisualBenchmarkConfig {
                total_rows: 10_000,
                batch_size: 100,
                dims: 4096,
                mode: VectorStoreMode::Resident,
            },
            current_rss_mb,
        )
        .expect("resident visual benchmark");

        let out_dir = std::path::PathBuf::from("/tmp").join(format!(
            "zebflow-visual-rust-prototype-bench-{}",
            unix_millis()
        ));
        std::fs::create_dir_all(&out_dir).expect("create benchmark report dir");
        write_report(&out_dir, &[count_only, resident]).expect("write benchmark report");
        println!("report={}", out_dir.join("report.html").display());
    }

    fn write_report(
        out_dir: &std::path::Path,
        reports: &[VisualBenchmarkReport],
    ) -> std::io::Result<()> {
        let json = serde_json::json!({
            "prototype": "visual_rust",
            "reports": reports.iter().map(report_json).collect::<Vec<_>>(),
        });
        std::fs::write(
            out_dir.join("summary.json"),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        let rows = reports
            .iter()
            .map(|report| {
                format!(
                    "<tr><td>{:?}</td><td>{}</td><td>{:.2}</td><td>{:.1}</td><td>{:.1}</td><td>{}</td><td>{:.2}</td></tr>",
                    report.config.mode,
                    report.rows,
                    report.elapsed.as_secs_f64(),
                    report.rows_per_second,
                    report.peak_rss_mb.unwrap_or_default(),
                    report.trace_entries,
                    report.observed_payload_bytes as f64 / (1024.0 * 1024.0),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let html = format!(
            "<!doctype html><meta charset='utf-8'><title>Visual Rust Prototype Benchmark</title>\
             <style>body{{font:14px system-ui;margin:32px;background:#f8fafc;color:#0f172a}}\
             table{{border-collapse:collapse;background:white}}td,th{{padding:8px 10px;border:1px solid #cbd5e1}}\
             th{{background:#e2e8f0}}</style><h1>Visual Rust Prototype Benchmark</h1>\
             <table><tr><th>mode</th><th>rows</th><th>seconds</th><th>rows/s</th><th>peak RSS MB</th><th>trace entries</th><th>observed payload MiB</th></tr>{rows}</table>"
        );
        std::fs::write(out_dir.join("report.html"), html)?;
        Ok(())
    }

    fn report_json(report: &VisualBenchmarkReport) -> serde_json::Value {
        serde_json::json!({
            "mode": format!("{:?}", report.config.mode),
            "total_rows": report.config.total_rows,
            "batch_size": report.config.batch_size,
            "dims": report.config.dims,
            "rows": report.rows,
            "batches": report.batches,
            "elapsed_s": report.elapsed.as_secs_f64(),
            "rows_per_second": report.rows_per_second,
            "trace_entries": report.trace_entries,
            "observed_payload_bytes": report.observed_payload_bytes,
            "peak_rss_mb": report.peak_rss_mb,
            "final_rss_mb": report.final_rss_mb,
        })
    }

    fn current_rss_mb() -> Option<f64> {
        let pid = std::process::id().to_string();
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let kb = text.trim().parse::<f64>().ok()?;
        Some(kb / 1024.0)
    }

    fn unix_millis() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }
}
