//! The security scan every uploaded library passes before anything is
//! installed (`rwe-library-manifest/README.md` § Project libraries). One
//! finding refuses the whole upload, naming the file and the reason. Each
//! check is its own function, so a new one is added without touching the rest.

use std::collections::BTreeMap;

use super::refuse;
use crate::platform::error::PlatformError;

/// The file kinds a library may carry, by extension. Anything else — a page
/// (`.html`), a source map, a program — is refused.
const ALLOWED: &[&str] = &[
    // code, styles, types
    "mjs", "js", "css", "d.ts", "d.mts",
    // data
    "json", "geojson", "csv", "tsv", "txt", "bin", "pbf", "mvt", "wasm",
    // 3D, images, fonts
    "glb", "gltf", "ktx2", "exr", "hdr", "png", "jpg", "jpeg", "webp", "avif", "gif", "svg",
    "woff", "woff2", "ttf", "otf",
];

/// Paths a build machine leaks into code it bundles.
const MACHINE_PATHS: &[&str] = &["/Users/", "/home/", "C:\\Users\\", "C:/Users/", "/private/var/folders/", "/private/tmp/"];

fn extension_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    for double in ["d.ts", "d.mts"] {
        if name.ends_with(&format!(".{double}")) {
            return double.to_string();
        }
    }
    name.rsplit_once('.').map(|(_, ext)| ext.to_string()).unwrap_or_default()
}

/// Programs for any system, whatever their name says.
fn is_program(bytes: &[u8]) -> bool {
    const MAGIC: &[&[u8]] = &[
        b"\x7fELF",
        b"MZ",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
        b"#!",
    ];
    MAGIC.iter().any(|magic| bytes.starts_with(magic))
}

fn check_kind(path: &str, bytes: &[u8], notices: &[String]) -> Result<(), String> {
    if path == "manifest.json" || notices.iter().any(|notice| notice == path) {
        return Ok(());
    }
    let ext = extension_of(path);
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(format!("a .{ext} file may not be in a library"));
    }
    if is_program(bytes) {
        return Err("it is a program, not code or data".to_string());
    }
    if ext == "wasm" && !bytes.starts_with(b"\0asm") {
        return Err("it is named .wasm but is not WebAssembly".to_string());
    }
    Ok(())
}

/// An SVG is an image that can carry script; one that does is refused.
fn check_svg(path: &str, bytes: &[u8]) -> Result<(), String> {
    if extension_of(path) != "svg" {
        return Ok(());
    }
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    let scripted = text.contains("<script") || text.contains("javascript:") || text.contains("<foreignobject")
        || text.split(|c: char| c.is_whitespace()).any(|word| word.starts_with("on") && word.contains('='));
    if scripted { Err("the image carries script".to_string()) } else { Ok(()) }
}

/// Whether `text` holds a path of the machine that built it: a file path
/// below a home folder (`<home>/<name>/<more>`), or one in the system's
/// temporary folders. A bare home folder with nothing below it is not a file
/// path: code compiled to WebAssembly with Emscripten carries the home of the
/// pretend Linux it runs in, and every such build has that text.
fn machine_path_in(text: &str) -> Option<&'static str> {
    const HOMES: &[&str] = &["/Users/", "/home/", "C:\\Users\\", "C:/Users/"];
    let ends_name = |c: char| matches!(c, '"' | '\'' | '`' | ' ' | ',' | ')' | '}' | ']' | ';' | '\n');
    for leak in MACHINE_PATHS {
        let is_home = HOMES.contains(leak);
        for (at, _) in text.match_indices(leak) {
            if !is_home {
                return Some(leak);
            }
            let rest = &text[at + leak.len()..];
            let name_len = rest.find(|c: char| c == '/' || c == '\\' || ends_name(c)).unwrap_or(rest.len());
            let after = &rest[name_len..];
            let more = after.starts_with('/') || after.starts_with('\\');
            if name_len > 0 && more && after.len() > 1 && !ends_name(after[1..].chars().next().unwrap_or('"')) {
                return Some(leak);
            }
        }
    }
    None
}

/// Code and data must not name the machine that built them.
fn check_machine_paths(path: &str, bytes: &[u8]) -> Result<(), String> {
    let ext = extension_of(path);
    if !matches!(ext.as_str(), "mjs" | "js" | "css" | "json" | "d.ts" | "d.mts" | "txt" | "svg" | "geojson") {
        return Ok(());
    }
    let text = String::from_utf8_lossy(bytes);
    match machine_path_in(&text) {
        Some(leak) => Err(format!("it names a path of the machine that built it ({leak}…); rebuild it")),
        None => Ok(()),
    }
}

/// Code loads only from its own library: never a script from another site,
/// never code made from text at run time.
fn check_code(path: &str, bytes: &[u8]) -> Result<(), String> {
    if !matches!(extension_of(path).as_str(), "mjs" | "js") {
        return Ok(());
    }
    let source = std::str::from_utf8(bytes).map_err(|_| "it is not UTF-8 text".to_string())?;
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    for (form, what) in [
        ("import(\"http", "loads code from another site"),
        ("import('http", "loads code from another site"),
        ("import(`http", "loads code from another site"),
        ("import(\"//", "loads code from another site"),
        ("import('//", "loads code from another site"),
        ("from\"http", "imports code from another site"),
        ("from'http", "imports code from another site"),
        ("from\"//", "imports code from another site"),
        ("from'//", "imports code from another site"),
        ("importScripts(", "loads scripts into a worker by address"),
    ] {
        if compact.contains(form) {
            return Err(format!("it {what}"));
        }
    }
    // The checker every page passes, for what can run made-up code.
    let policy = crate::rwe::core::config::SecurityPolicy {
        import_allowlist: Vec::new(),
        blocked_globals: vec!["eval".to_string(), "Function".to_string(), "globalThis.Function".to_string()],
        allow_dynamic_import: true,
        allow_raw_html: true,
        network_allowlist: Vec::new(),
    };
    crate::rwe::core::security::analyze(source, &policy).map(|_| ()).map_err(|err| format!("{} ({})", err.message, err.code))
}

/// Runs every check on every file; the first finding refuses the upload.
pub(super) fn scan(files: &BTreeMap<String, Vec<u8>>, notices: &[String]) -> Result<(), PlatformError> {
    type Check = fn(&str, &[u8]) -> Result<(), String>;
    let checks: [Check; 3] = [check_svg, check_machine_paths, check_code];
    for (path, bytes) in files {
        let finding = check_kind(path, bytes, notices).err().or_else(|| checks.iter().find_map(|check| check(path, bytes).err()));
        if let Some(reason) = finding {
            return Err(refuse(format!("security scan: '{path}': {reason}")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(path: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        scan(&BTreeMap::from([(path.to_string(), bytes.to_vec())]), &["THIRD-PARTY-NOTICES.txt".to_string()])
    }

    #[test]
    fn plain_library_files_pass() {
        for (path, bytes) in [
            ("0.1.0/runtime/world.mjs", &b"import{a}from\"./chunk-1.mjs\";export const w=()=>import(\"./lazy.mjs\");const u=new URL(\"./x.worker.mjs\",import.meta.url);"[..]),
            ("0.1.0/data/terrain.bin", b"\x00\x01\x02"),
            ("0.1.0/runtime/m.wasm", b"\0asm\x01\0\0\0"),
            ("0.1.0/types/world.d.ts", b"export declare const w: () => void;"),
            ("0.1.0/data/icon.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0\"/></svg>"),
            ("THIRD-PARTY-NOTICES.txt", b"MIT"),
        ] {
            one(path, bytes).unwrap_or_else(|err| panic!("{path}: {}", err.message));
        }
    }

    #[test]
    fn anything_that_could_run_or_leak_is_refused() {
        for (path, bytes, word) in [
            ("0.1.0/runtime/a.mjs", &b"const f = eval(\"1\");"[..], "eval"),
            ("0.1.0/runtime/a.mjs", b"const f = new Function(\"return 1\");", "Function"),
            ("0.1.0/runtime/a.mjs", b"import(\"https://cdn.example.com/x.mjs\")", "another site"),
            ("0.1.0/runtime/a.mjs", b"import x from 'https://cdn.example.com/x.mjs'", "another site"),
            ("0.1.0/runtime/w.worker.mjs", b"importScripts(\"https://cdn.example.com/x.js\")", "worker"),
            ("0.1.0/runtime/a.mjs.map", b"{}", ".map"),
            ("0.1.0/index.html", b"<html></html>", ".html"),
            ("0.1.0/data/tool.bin", b"\x7fELF\x02\x01", "program"),
            ("0.1.0/runtime/x.wasm", b"MZ\x90\x00", "program"),
            ("0.1.0/data/fake.wasm", b"nope", "WebAssembly"),
            ("0.1.0/data/icon.svg", b"<svg onload=\"alert(1)\"></svg>", "script"),
            ("0.1.0/data/LICENSE", b"text", "may not"),
        ] {
            let err = one(path, bytes).expect_err(path);
            assert!(err.message.contains(word), "{path}: {}", err.message);
        }
        // A bare home folder (Emscripten's pretend Linux) is not a file path;
        // a file below a home beside it still is.
        let bare = format!("var ENV={{USER:\"someone\",HOME:\"{}someone\"}};", MACHINE_PATHS[1]);
        one("0.1.0/runtime/w.mjs", bare.as_bytes()).expect("a bare home folder");
        let both = format!("{bare}const p=\"{}someone/src/a.ts\";", MACHINE_PATHS[1]);
        assert!(one("0.1.0/runtime/w.mjs", both.as_bytes()).is_err(), "a file below a home is still refused");
        // Every path a build machine leaks, built from the list itself.
        for leak in MACHINE_PATHS {
            let code = format!("const p = \"{leak}someone/src/a.ts\";");
            let err = one("0.1.0/runtime/a.mjs", code.as_bytes()).expect_err(leak);
            assert!(err.message.contains("machine"), "{leak}: {}", err.message);
        }
    }
}
