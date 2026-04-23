use std::fs;

use serde_json::Value;

use super::SourceAdapter;

pub struct GeoJsonFileSource;

impl SourceAdapter for GeoJsonFileSource {
    fn load(path: &str) -> Result<Value, String> {
        let raw =
            fs::read_to_string(path).map_err(|err| format!("failed reading geojson file: {err}"))?;
        serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("failed parsing geojson file: {err}"))
    }
}
