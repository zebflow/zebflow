use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

pub fn stream_feature_collection_from_path(
    path: &Path,
    mut on_feature: impl FnMut(Value) -> Result<(), String>,
) -> Result<(), String> {
    let file = File::open(path)
        .map_err(|err| format!("failed reading geojson file {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut de = serde_json::Deserializer::from_reader(reader);
    FeatureCollectionSeed {
        on_feature: &mut on_feature,
    }
    .deserialize(&mut de)
    .map_err(|err| format!("failed parsing geojson feature collection: {err}"))?;
    Ok(())
}

struct FeatureCollectionSeed<'a, F> {
    on_feature: &'a mut F,
}

impl<'de, F> DeserializeSeed<'de> for FeatureCollectionSeed<'_, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de, F> Visitor<'de> for FeatureCollectionSeed<'_, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a GeoJSON FeatureCollection object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut saw_type = false;
        let mut saw_features = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => {
                    let value = map.next_value::<String>()?;
                    if value != "FeatureCollection" {
                        return Err(serde::de::Error::custom(
                            "source is not a GeoJSON FeatureCollection",
                        ));
                    }
                    saw_type = true;
                }
                "features" => {
                    map.next_value_seed(FeaturesSeed {
                        on_feature: self.on_feature,
                    })?;
                    saw_features = true;
                }
                _ => {
                    let _ = map.next_value::<IgnoredAny>()?;
                }
            }
        }
        if !saw_type {
            return Err(serde::de::Error::custom(
                "GeoJSON missing type=FeatureCollection",
            ));
        }
        if !saw_features {
            return Err(serde::de::Error::custom("GeoJSON missing features array"));
        }
        Ok(())
    }
}

struct FeaturesSeed<'a, F> {
    on_feature: &'a mut F,
}

impl<'de, F> DeserializeSeed<'de> for FeaturesSeed<'_, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, F> Visitor<'de> for FeaturesSeed<'_, F>
where
    F: FnMut(Value) -> Result<(), String>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a GeoJSON features array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(feature) = seq.next_element::<Value>()? {
            (self.on_feature)(feature).map_err(serde::de::Error::custom)?;
        }
        Ok(())
    }
}
