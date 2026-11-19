//! The `.tar.gz` a project library arrives in, read by Zebflow itself: plain
//! files on plain relative paths, within size and count limits, nothing else.

use std::collections::BTreeMap;
use std::io::Read;

use super::refuse;
use crate::platform::error::PlatformError;

/// The most an upload may hold once unpacked: a small archive that unpacks
/// to gigabytes is refused before it is read.
const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;
/// The most files one library may carry.
const MAX_FILES: usize = 4000;

/// The plain files of a `.tar.gz`, by path. Anything that is not a plain file
/// or a folder is refused, and so is a path that is absolute, climbs with
/// `..`, uses `\`, or appears twice.
pub(super) fn read_tar_gz(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, PlatformError> {
    let mut unpacked = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_UNPACKED_BYTES + 1)
        .read_to_end(&mut unpacked)
        .map_err(|err| refuse(format!("the upload is not a .tar.gz: {err}")))?;
    if unpacked.len() as u64 > MAX_UNPACKED_BYTES {
        return Err(refuse(format!("the upload unpacks to more than {} MiB", MAX_UNPACKED_BYTES / 1024 / 1024)));
    }
    let field = |header: &[u8], from: usize, to: usize| -> String {
        let raw = &header[from..to];
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..end]).to_string()
    };
    let mut files = BTreeMap::new();
    let mut at = 0usize;
    while at + 512 <= unpacked.len() {
        let header = &unpacked[at..at + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let mut name = field(header, 0, 100);
        if &header[257..262] == b"ustar" {
            let prefix = field(header, 345, 500);
            if !prefix.is_empty() {
                name = format!("{prefix}/{name}");
            }
        }
        let size_text = field(header, 124, 136);
        let size = u64::from_str_radix(size_text.trim(), 8).map_err(|_| refuse(format!("'{name}' has an unreadable size")))?;
        let body = at + 512;
        let next = body + size.div_ceil(512) as usize * 512;
        if body + size as usize > unpacked.len() {
            return Err(refuse(format!("'{name}' is cut short")));
        }
        match header[156] {
            b'0' | 0 => {
                let clean = name.trim_start_matches("./").to_string();
                let bad = clean.is_empty()
                    || clean.starts_with('/')
                    || clean.contains('\\')
                    || clean.split('/').any(|segment| segment.is_empty() || segment == "." || segment == "..")
                    || clean.chars().any(char::is_control);
                if bad {
                    return Err(refuse(format!("'{name}' is not a plain relative path")));
                }
                if files.insert(clean.clone(), unpacked[body..body + size as usize].to_vec()).is_some() {
                    return Err(refuse(format!("'{clean}' appears twice")));
                }
                if files.len() > MAX_FILES {
                    return Err(refuse(format!("the upload holds more than {MAX_FILES} files")));
                }
            }
            b'5' => {}
            other => {
                return Err(refuse(format!(
                    "'{name}' is not a plain file (tar type '{}'); upload the .tar.gz zeb-bundler writes",
                    other as char
                )));
            }
        }
        at = next;
    }
    Ok(files)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal ustar `.tar.gz` of `(path, type, bytes)`.
    pub(crate) fn tar_gz(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (path, kind, bytes) in entries {
            let mut header = [0u8; 512];
            header[..path.len()].copy_from_slice(path.as_bytes());
            header[100..107].copy_from_slice(b"0000644");
            header[124..135].copy_from_slice(format!("{:011o}", bytes.len()).as_bytes());
            header[136..147].copy_from_slice(b"00000000000");
            header[156] = *kind;
            header[257..262].copy_from_slice(b"ustar");
            header[148..156].copy_from_slice(b"        ");
            let sum: u32 = header.iter().map(|b| *b as u32).sum();
            header[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
            out.extend_from_slice(&header);
            out.extend_from_slice(bytes);
            out.resize(out.len().div_ceil(512) * 512, 0);
        }
        out.resize(out.len() + 1024, 0);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, &out).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn only_plain_files_on_plain_paths_are_read() {
        let files = read_tar_gz(&tar_gz(&[("manifest.json", b'0', b"{}"), ("0.1.0/", b'5', b""), ("0.1.0/runtime/a.mjs", b'0', b"x")])).expect("read");
        assert_eq!(files.keys().collect::<Vec<_>>(), vec!["0.1.0/runtime/a.mjs", "manifest.json"]);
        for (path, kind) in [("../escape.mjs", b'0'), ("/abs.mjs", b'0'), ("a/../b.mjs", b'0'), ("link.mjs", b'2'), ("hard.mjs", b'1'), ("pax", b'x')] {
            assert!(read_tar_gz(&tar_gz(&[(path, kind, b"x")])).is_err(), "{path}");
        }
        assert!(read_tar_gz(&tar_gz(&[("a.mjs", b'0', b"1"), ("a.mjs", b'0', b"2")])).is_err(), "twice");
        assert!(read_tar_gz(b"not gzip").is_err());
    }
}
