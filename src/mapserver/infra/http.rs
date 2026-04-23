use std::collections::HashMap;

pub fn parse_bbox_param(params: &HashMap<String, String>) -> Result<Option<[f64; 4]>, String> {
    let Some(raw) = params.get("bbox").map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let parts = raw
        .split(',')
        .map(str::trim)
        .map(|s| s.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "invalid bbox".to_string())?;
    if parts.len() != 4 {
        return Err("invalid bbox".to_string());
    }
    Ok(Some([parts[0], parts[1], parts[2], parts[3]]))
}

pub fn parse_limit_param(
    params: &HashMap<String, String>,
    default_limit: usize,
) -> Result<usize, String> {
    let Some(raw) = params.get("limit").map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(default_limit);
    };
    raw.parse::<usize>().map_err(|_| "invalid limit".to_string())
}
