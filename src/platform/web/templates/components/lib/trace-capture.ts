/** Shared logging form contract. Empty values inherit; zero is meaningful for arrays. */
export const traceCaptureFields = [
  { key: "array_sample_count", label: "Array sample count", min: 0, max: 1000000, defaultValue: 6, help: "First N items of every nested array. 0 keeps all array items; other limits still apply." },
  { key: "max_string_chars", label: "Max string characters", min: 1, max: 1048576, defaultValue: 8192, help: "Maximum captured characters per string." },
  { key: "max_depth", label: "Max nesting depth", min: 1, max: 64, defaultValue: 8, help: "Deeper objects and arrays are summarized." },
  { key: "max_node_bytes", label: "Max captured bytes per node", min: 256, max: 16777216, defaultValue: 65536, help: "Captured config, input, and output budget per node; fixed trace metadata is excluded." },
  { key: "max_run_bytes", label: "Max captured bytes per run", min: 256, max: 67108864, defaultValue: 1048576, help: "Captured data budget per invocation; fixed trace metadata is excluded." },
];

export function traceCaptureFormValues(config) {
  const result = {};
  for (const field of traceCaptureFields) result[field.key] = config?.[field.key] == null ? "" : String(config[field.key]);
  return result;
}

/** Validate instead of silently clamping; undefined keys stay absent on save. */
export function traceCaptureFormConfig(values) {
  const result = {};
  for (const field of traceCaptureFields) {
    const raw = String(values?.[field.key] ?? "").trim();
    if (!raw) continue;
    const value = Number(raw);
    if (!Number.isInteger(value) || value < field.min || value > field.max) {
      throw new Error(`${field.label} must be an integer from ${field.min} to ${field.max}.`);
    }
    result[field.key] = value;
  }
  return result;
}
