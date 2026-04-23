use serde_json::Value;

pub mod geojson_file;

pub trait SourceAdapter {
    fn load(path: &str) -> Result<Value, String>;
}
