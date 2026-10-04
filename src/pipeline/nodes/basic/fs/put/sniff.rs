//! What a file given to `fs.file.put --from` really is.
//!
//! The browser's word for a file is a claim; its first bytes are evidence.
//! The type is read from the bytes (the `infer` crate, plus GLB and Ogg
//! Theora by hand), the claim must agree with it, the type must be one the
//! node's `--accept` words allow, and the stored extension follows the
//! detected type. Text formats (SVG, JSON, CSV) have no magic bytes, so for
//! them alone the claim is taken — and still checked against `--accept`.

use crate::pipeline::PipelineError;
use crate::pipeline::nodes::shared::limits::choice;

pub const ACCEPT_CODE: &str = "FW_NODE_FS_FILE_PUT_ACCEPT";

/// One `--accept` word: a family of content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accept {
    Image,
    Pdf,
    Csv,
    Json,
    Glb,
    Audio,
    Video,
    Archive,
}

/// The closed `--accept` words, in the order the help lists them.
pub const ACCEPT_WORDS: &[&str] = &["image", "pdf", "csv", "json", "glb", "audio", "video", "archive"];

impl Accept {
    fn parse(word: &str) -> Result<Self, PipelineError> {
        Ok(match choice(word, ACCEPT_WORDS, "", "--accept", ACCEPT_CODE)? {
            "image" => Self::Image,
            "pdf" => Self::Pdf,
            "csv" => Self::Csv,
            "json" => Self::Json,
            "glb" => Self::Glb,
            "audio" => Self::Audio,
            "video" => Self::Video,
            "archive" => Self::Archive,
            _ => return Err(PipelineError::new(ACCEPT_CODE, "--accept needs a word")),
        })
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Pdf => "pdf",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Glb => "glb",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Archive => "archive",
        }
    }

    fn mime_types(self) -> &'static [&'static str] {
        match self {
            Self::Image => &[
                "image/jpeg", "image/png", "image/webp", "image/gif", "image/bmp", "image/tiff",
                "image/svg+xml", "image/avif", "image/heic", "image/heif",
            ],
            Self::Pdf => &["application/pdf"],
            Self::Csv => &["text/csv", "text/plain"],
            Self::Json => &["application/json", "text/json"],
            Self::Glb => &["model/gltf-binary"],
            Self::Audio => &["audio/mpeg", "audio/wav", "audio/ogg", "audio/mp4"],
            Self::Video => &["video/mp4", "video/webm", "video/ogg", "video/x-m4v"],
            Self::Archive => &[
                "application/zip", "application/gzip", "application/x-tar", "application/x-bzip2",
                "application/x-7z-compressed", "application/x-rar-compressed",
                "application/vnd.android.package-archive", "application/java-archive",
            ],
        }
    }
}

/// The `--accept` words as given; none is `image`. An unknown word is
/// refused, never dropped.
pub fn parse_accept(words: &[String]) -> Result<Vec<Accept>, PipelineError> {
    let mut kinds = Vec::new();
    for word in words.iter().flat_map(|w| w.split(',')).map(str::trim).filter(|w| !w.is_empty()) {
        let kind = Accept::parse(word)?;
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        kinds.push(Accept::Image);
    }
    Ok(kinds)
}

/// The content type of `bytes` as `fs.file.put --from` stores it: detected
/// from the bytes, the browser's claim agreeing, inside `accept`.
pub fn effective_mime(bytes: &[u8], claimed: &str, accept: &[Accept]) -> Result<String, PipelineError> {
    let detected = detect_binary_mime(bytes).map(normalized_mime);
    let mime = match detected.as_deref() {
        Some(mime) => mime.to_string(),
        // Text has no magic bytes: only a text type may be taken on the claim.
        None => {
            let claimed_mime = normalized_mime(claimed);
            const TEXT: &[&str] = &["image/svg+xml", "application/json", "text/json", "text/csv", "text/plain"];
            if !TEXT.contains(&claimed_mime.as_str()) {
                return Err(PipelineError::new(
                    ACCEPT_CODE,
                    format!("the file's type could not be read from its content (it claims '{claimed}')"),
                ));
            }
            claimed_mime
        }
    };
    if !accept.iter().any(|kind| kind.mime_types().iter().any(|m| normalized_mime(m) == mime)) {
        let words: Vec<&str> = accept.iter().map(|kind| kind.word()).collect();
        return Err(PipelineError::new(
            ACCEPT_CODE,
            format!("the file is '{mime}', which --accept {} does not allow", words.join(",")),
        ));
    }
    if let Some(detected) = detected.as_deref()
        && !claim_matches_detected(claimed, detected)
    {
        return Err(PipelineError::new(
            ACCEPT_CODE,
            format!("the file claims '{claimed}' but its content is '{detected}'"),
        ));
    }
    Ok(mime)
}

/// The extensions a content type is stored under, the usual one first.
/// Empty for a type with no extension of its own.
pub fn extensions(mime: &str) -> &'static [&'static str] {
    match normalized_mime(mime).as_str() {
        "image/jpeg" => &["jpg", "jpeg"],
        "image/png" => &["png"],
        "image/webp" => &["webp"],
        "image/gif" => &["gif"],
        "image/bmp" => &["bmp"],
        "image/tiff" => &["tif", "tiff"],
        "image/svg+xml" => &["svg"],
        "image/avif" => &["avif"],
        "image/heic" => &["heic"],
        "image/heif" => &["heif", "heic"],
        "application/pdf" => &["pdf"],
        "text/plain" => &["txt", "csv", "text"],
        "text/csv" => &["csv"],
        "application/json" | "text/json" => &["json", "geojson"],
        "application/zip" => &["zip", "apk", "jar"],
        "video/mp4" => &["mp4", "m4v"],
        "video/webm" => &["webm"],
        "video/ogg" => &["ogv", "ogg"],
        "video/x-m4v" => &["m4v"],
        "audio/mpeg" => &["mp3"],
        "audio/wav" => &["wav"],
        "audio/ogg" => &["ogg", "oga", "opus"],
        "audio/mp4" => &["m4a", "mp4"],
        "model/gltf-binary" => &["glb"],
        "application/gzip" | "application/x-gzip" => &["gz", "tgz"],
        "application/x-tar" => &["tar"],
        "application/x-bzip2" => &["bz2"],
        "application/x-7z-compressed" => &["7z"],
        "application/x-rar-compressed" => &["rar"],
        "application/vnd.android.package-archive" => &["apk"],
        "application/java-archive" => &["jar"],
        _ => &[],
    }
}

/// The extension a file of `mime` is stored under: the original name's when
/// the type has that extension (`photo.jpeg` stays `jpeg`), else the type's
/// usual one. A name never chooses an extension its content disagrees with.
pub fn stored_extension(original_name: &str, mime: &str) -> &'static str {
    let allowed = extensions(mime);
    let original = extension_of(original_name);
    allowed
        .iter()
        .find(|ext| Some(**ext) == original.as_deref())
        .or_else(|| allowed.first())
        .copied()
        .unwrap_or("")
}

/// `true` when `ext` may name a file of `mime`. A type with no known
/// extension has nothing to disagree with.
pub fn extension_agrees(ext: &str, mime: &str) -> bool {
    let allowed = extensions(mime);
    allowed.is_empty() || allowed.contains(&ext.to_ascii_lowercase().as_str())
}

/// The lowercase alphanumeric extension of a name, at most 10 characters.
pub fn extension_of(name: &str) -> Option<String> {
    let ext = std::path::Path::new(name.trim()).extension()?.to_str()?;
    let ext: String = ext.chars().filter(|c| c.is_ascii_alphanumeric()).take(10).collect::<String>().to_ascii_lowercase();
    (!ext.is_empty()).then_some(ext)
}

pub fn normalized_mime(mime: &str) -> String {
    let raw = mime.split(';').next().unwrap_or("").trim().to_lowercase();
    match raw.as_str() {
        "image/jpg" => "image/jpeg".to_string(),
        "audio/x-wav" => "audio/wav".to_string(),
        "audio/m4a" => "audio/mp4".to_string(),
        "application/x-gzip" => "application/gzip".to_string(),
        // GeoJSON is JSON: the map nodes read it from a `.geojson` key.
        "application/geo+json" => "application/json".to_string(),
        _ => raw,
    }
}

/// The type the bytes declare, or `None` for content with no signature.
pub fn detect_binary_mime(bytes: &[u8]) -> Option<&'static str> {
    if is_glb(bytes) {
        return Some("model/gltf-binary");
    }
    if is_ogg_theora(bytes) {
        return Some("video/ogg");
    }
    infer::get(bytes).map(|kind| kind.mime_type())
}

fn is_glb(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return false;
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let declared_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    (version == 1 || version == 2) && declared_len >= 12 && declared_len <= bytes.len()
}

fn is_ogg_theora(bytes: &[u8]) -> bool {
    if bytes.len() < 35 || &bytes[0..4] != b"OggS" {
        return false;
    }
    let page_segments = bytes[26] as usize;
    let payload_start = 27 + page_segments;
    bytes.get(payload_start..payload_start + 7) == Some(&b"\x80theora"[..])
}

/// A generic claim (`application/octet-stream`, none) defers to the bytes; a
/// specific one must name what the bytes are. ZIP-based formats (APK, JAR)
/// carry ZIP's signature, so their own words agree with it.
fn claim_matches_detected(claimed: &str, detected: &str) -> bool {
    let claimed = normalized_mime(claimed);
    let detected = normalized_mime(detected);
    if claimed.is_empty() || claimed == "application/octet-stream" || claimed == "binary/octet-stream" || claimed == detected {
        return true;
    }
    detected == "application/zip"
        && ["application/vnd.android.package-archive", "application/java-archive"].contains(&claimed.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x02\0\0\0";

    #[test]
    fn accept_words_are_singular_and_closed() {
        assert_eq!(parse_accept(&[]).unwrap(), vec![Accept::Image]);
        assert_eq!(parse_accept(&["image".into(), "PDF".into()]).unwrap(), vec![Accept::Image, Accept::Pdf]);
        assert_eq!(parse_accept(&["image,pdf".into()]).unwrap(), vec![Accept::Image, Accept::Pdf]);
        for bad in ["images", "documents", "jpg"] {
            let err = parse_accept(&[bad.into()]).unwrap_err();
            assert_eq!(err.code, ACCEPT_CODE);
            assert!(err.message.contains("must be one of image, pdf"), "{}", err.message);
        }
    }

    #[test]
    fn audio_aliases_are_accepted() {
        assert!(effective_mime(b"", "audio/x-wav", &[Accept::Audio]).is_err(), "no signature, not text");
        assert_eq!(normalized_mime("audio/x-wav"), "audio/wav");
        assert_eq!(normalized_mime("audio/m4a"), "audio/mp4");
        assert!(Accept::Audio.mime_types().contains(&"audio/wav"));
    }

    /// A GeoJSON upload is JSON, kept under its own `.geojson` name.
    #[test]
    fn geojson_is_json_under_its_own_extension() {
        let geo = br#"{"type":"FeatureCollection","features":[]}"#;
        assert_eq!(effective_mime(geo, "application/geo+json", &[Accept::Json]).unwrap(), "application/json");
        assert_eq!(stored_extension("places.geojson", "application/json"), "geojson");
        assert_eq!(stored_extension("places", "application/json"), "json");
    }

    #[test]
    fn detects_glb_by_header() {
        let mut bytes = b"glTF".to_vec();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(12u32).to_le_bytes());
        assert_eq!(detect_binary_mime(&bytes), Some("model/gltf-binary"));
    }

    #[test]
    fn detects_ogg_theora_as_video() {
        let mut bytes = vec![0; 35];
        bytes[0..4].copy_from_slice(b"OggS");
        bytes[26] = 1;
        bytes[27] = 7;
        bytes[28..35].copy_from_slice(b"\x80theora");
        assert_eq!(detect_binary_mime(&bytes), Some("video/ogg"));
    }

    #[test]
    fn the_content_decides_and_the_claim_must_agree() {
        assert_eq!(effective_mime(PNG, "image/png", &[Accept::Image]).unwrap(), "image/png");
        assert_eq!(effective_mime(PNG, "application/octet-stream", &[Accept::Image]).unwrap(), "image/png");
        // A PNG claiming to be a JPEG is refused; so is a PNG where only PDFs are accepted.
        assert!(effective_mime(PNG, "image/jpeg", &[Accept::Image]).unwrap_err().message.contains("claims"));
        assert!(effective_mime(PNG, "image/png", &[Accept::Pdf]).unwrap_err().message.contains("--accept pdf"));
        // Text is taken on its claim, and only text.
        assert_eq!(effective_mime(b"a,b\n1,2\n", "text/csv", &[Accept::Csv]).unwrap(), "text/csv");
        assert!(effective_mime(b"a,b\n", "image/png", &[Accept::Image]).is_err());
    }

    #[test]
    fn the_extension_follows_the_detected_type() {
        assert_eq!(stored_extension("upload", "model/gltf-binary"), "glb");
        assert_eq!(stored_extension("photo.jpeg", "image/jpeg"), "jpeg");
        assert_eq!(stored_extension("photo.exe", "image/jpeg"), "jpg");
        assert_eq!(stored_extension("upload", "audio/x-wav"), "wav");
        assert_eq!(stored_extension("upload", "audio/m4a"), "m4a");
        assert_eq!(stored_extension("app.apk", "application/zip"), "apk");
        assert_eq!(stored_extension("upload", "application/x-unknown"), "");
        assert!(extension_agrees("JPG", "image/jpeg"));
        assert!(!extension_agrees("png", "image/jpeg"));
    }
}
