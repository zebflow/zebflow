//! Embedded platform templates and official library assets.
//!
//! `blessed/` is not a watched path, so a hand-edited library bundle (the
//! graphui canvas, for one) only reaches a running dev server when something
//! under `src/` changes too. (Touched 2026-09-20: note colour picker, resizable
//! input widgets, undeclared `:error` pins kept through a save. Touched
//! 2026-09-21: `retry` / `error_routed` run badges — an orange ring with the
//! attempt count, never red.)

/// One embedded file shipped inside the binary.
pub struct EmbeddedAsset {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

const BRAND_LOGO_SVG: &[u8] = include_bytes!("assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("assets/branding/logo.png");
const BRAND_FAVICON_SVG: &[u8] = include_bytes!("assets/branding/favicon.svg");
const BRAND_FAVICON_ICO: &[u8] = include_bytes!("assets/branding/favicon.ico");
const BRAND_FAVICON_16_PNG: &[u8] = include_bytes!("assets/branding/favicon-16.png");
const BRAND_FAVICON_32_PNG: &[u8] = include_bytes!("assets/branding/favicon-32.png");
const BRAND_APPLE_TOUCH_ICON_PNG: &[u8] = include_bytes!("assets/branding/apple-touch-icon.png");
// Kept in step with the same concat in `mod.rs`. The two drifted once already:
// this copy was missing fonts.css, so a static export shipped stylesheets whose
// @font-face rules had gone missing and rendered in a fallback face.
const PLATFORM_MAIN_CSS: &str = concat!(
    include_str!("templates/styles/fonts.css"),
    "\n\n",
    include_str!("templates/styles/main.css"),
    "\n\n",
    include_str!("templates/pages/project-studio/styles.css"),
);
const PLATFORM_DB_SUITE_CSS: &str = include_str!("templates/styles/db-suite.css");
const PLATFORM_DB_CONNECTIONS_CSS: &str = include_str!("templates/styles/db-connections.css");
const PLATFORM_DEVICONS_CSS: &str = include_str!("templates/styles/devicons.css");

const FONT_HANKENGROTESK_CYRILLIC_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-cyrillic-ext.woff2");
const FONT_HANKENGROTESK_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-latin-ext.woff2");
const FONT_HANKENGROTESK_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-latin.woff2");
const FONT_HANKENGROTESK_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-vietnamese.woff2");
const FONT_JETBRAINSMONO_CYRILLIC_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-cyrillic-ext.woff2");
const FONT_JETBRAINSMONO_CYRILLIC_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-cyrillic.woff2");
const FONT_JETBRAINSMONO_GREEK_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-greek.woff2");
const FONT_JETBRAINSMONO_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-latin-ext.woff2");
const FONT_JETBRAINSMONO_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-latin.woff2");
const FONT_JETBRAINSMONO_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-vietnamese.woff2");
const FONT_SPACEGROTESK_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-latin-ext.woff2");
const FONT_SPACEGROTESK_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-latin.woff2");
const FONT_SPACEGROTESK_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-vietnamese.woff2");


/// The libraries the Studio's own pages load.
///
/// A separate copy from `PLATFORM_LIBRARY_ASSETS` on purpose. That list is the
/// hub's catalogue — the bytes a project installs — and the Studio used to read
/// straight out of it. So editing a blessed package silently changed the
/// Studio, and removing one broke the build: deleting
/// `blessed/rwe-libraries/icons/` failed compilation with `couldn't read …
/// include_bytes!`, which is a hub decision reaching into the platform.
///
/// These are copies under `src/platform/web/vendor/`. The hub is free to bump,
/// patch or drop a package without the Studio moving underneath it, and the
/// Studio pins what it was tested against.
pub const PLATFORM_VENDOR_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/exports.json",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/keywords.json",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/entry.mjs",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/runtime/entry.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/wrappers/CodeEditor.tsx",
        bytes: include_bytes!("vendor/zeb/codemirror/0.1/wrappers/CodeEditor.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/manifest.json",
        bytes: include_bytes!("vendor/zeb/codemirror/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/package.yaml",
        bytes: include_bytes!("vendor/zeb/codemirror/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/exports.json",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/keywords.json",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/runtime/deckgl.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.patched.mjs",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/runtime/deckgl.patched.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/wrappers/DeckMap.tsx",
        bytes: include_bytes!("vendor/zeb/deckgl/0.1/wrappers/DeckMap.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/manifest.json",
        bytes: include_bytes!("vendor/zeb/deckgl/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/package.yaml",
        bytes: include_bytes!("vendor/zeb/deckgl/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/exports.json",
        bytes: include_bytes!("vendor/zeb/markdown/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/keywords.json",
        bytes: include_bytes!("vendor/zeb/markdown/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/markdown/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/runtime/markdown.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/markdown/0.1/runtime/markdown.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/wrappers/Markdown.tsx",
        bytes: include_bytes!("vendor/zeb/markdown/0.1/wrappers/Markdown.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/manifest.json",
        bytes: include_bytes!("vendor/zeb/markdown/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/package.yaml",
        bytes: include_bytes!("vendor/zeb/markdown/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/pdf/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/runtime/pdf.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/pdf/0.1/runtime/pdf.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/manifest.json",
        bytes: include_bytes!("vendor/zeb/pdf/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/package.yaml",
        bytes: include_bytes!("vendor/zeb/pdf/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/exports.json",
        bytes: include_bytes!("vendor/zeb/use/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/keywords.json",
        bytes: include_bytes!("vendor/zeb/use/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/use/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/runtime/use.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/use/0.1/runtime/use.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/use/manifest.json",
        bytes: include_bytes!("vendor/zeb/use/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/package.yaml",
        bytes: include_bytes!("vendor/zeb/use/package.yaml"),
    },
];

pub const PLATFORM_NODE_ICON_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "manifest.json",
        bytes: include_bytes!("assets/node-icons/manifest.json"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ai.agent.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ai.agent.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ai.tts.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ai.tts.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.auth.token.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.auth.token.create.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.browser.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.browser.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.crypto.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.crypto.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.compress.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.compress.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.copy.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.copy.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.decompress.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.decompress.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.delete.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.delete.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.head.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.head.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.list.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.list.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.mkdir.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.mkdir.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.move.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.move.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.pdf.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.pdf.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.put.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.put.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.save.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.save.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.image.thumbnail.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.image.thumbnail.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.fs.svg.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.svg.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.function.call.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.function.call.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.geo.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.geo.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.geo.inspect.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.geo.inspect.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.http.request.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.http.request.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.collect.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.collect.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.foreach.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.foreach.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.if.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.if.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.match.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.match.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.reduce.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.reduce.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.logic.retry.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.logic.retry.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.set.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.set.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.del.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.del.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.exists.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.exists.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.expire.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.expire.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.incr.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.incr.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.kv.publish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.kv.publish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.pg.query.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.pg.query.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.sekejap.query.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.sekejap.query.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.table.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.table.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.table.query.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.table.query.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.script.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.script.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.sqlite.mutate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.sqlite.mutate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.sqlite.query.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.sqlite.query.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.function.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.function.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.manual.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.manual.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.mapserver.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.mapserver.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.mcp.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.mcp.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.memsubscribe.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.memsubscribe.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ms.publish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ms.publish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ms.unpublish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ms.unpublish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ms.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ms.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ms.list.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ms.list.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.kv.subscribe.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.kv.subscribe.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.schedule.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.schedule.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.webhook.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.webhook.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.weberror.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.weberror.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.ws.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.ws.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.trigger.ws.client.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.trigger.ws.client.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.web.docs.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.web.docs.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.web.response.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.web.response.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.web.static.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.web.static.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ws.client.send.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ws.client.send.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ws.emit.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ws.emit.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ws.sync_state.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ws.sync_state.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.ai.embedding.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.ai.embedding.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.auth.token.verify.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.auth.token.verify.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.concept.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.concept.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.mail.send.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.mail.send.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.text.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.text.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.number.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.number.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.boolean.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.boolean.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.json.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.json.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.file.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.file.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.files.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.files.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.image.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.image.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.audio.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.audio.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.input.video.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.input.video.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/n.sekejap.insert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.sekejap.insert.svg"),
    },
];

pub fn platform_node_icon_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_NODE_ICON_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

// PLATFORM_COMPOSITE_NODE_ASSETS — auto-generated at build time from src/pipeline/nodes/bundled/.
// Do not edit manually; add files to a bundle directory and recompile.
include!(concat!(env!("OUT_DIR"), "/node_bundles_gen.rs"));

pub fn platform_composite_node_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_COMPOSITE_NODE_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

// PLATFORM_TEMPLATE_ASSETS — auto-generated at build time from src/platform/web/templates/.
// Do not edit manually; add files to that directory and recompile.
include!(concat!(env!("OUT_DIR"), "/platform_templates_gen.rs"));

// PLATFORM_SOURCE_LIBRARY_ASSETS — auto-generated at build time from
// blessed/source-libraries/. `zeb/ui/0.1/src/button.tsx` and friends: source
// the RWE compiler inlines into a page, not a runtime bundle.
include!(concat!(env!("OUT_DIR"), "/source_libraries_gen.rs"));

// PLATFORM_SKILL_ASSETS — auto-generated at build time from blessed/skills/.
// `zebflow-basic/SKILL.md` and friends: the skills every project's agent is
// shown without cloning; a project's own `skills/<name>/` shadows one by name.
include!(concat!(env!("OUT_DIR"), "/skills_gen.rs"));

// PLATFORM_SKILL_EXTRA_ASSETS — auto-generated from blessed/skill-extras/.
// The optional skills: shelf packages a project adds, never listed until it does.
include!(concat!(env!("OUT_DIR"), "/skill_extras_gen.rs"));

pub const PLATFORM_LIBRARY_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "zeb/d3/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/runtime/d3.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/0.1/runtime/d3.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/deckgl/0.1/runtime/deckgl.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.patched.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/deckgl/0.1/runtime/deckgl.patched.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/wrappers/DeckMap.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/0.1/wrappers/DeckMap.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/codemirror/0.1/runtime/codemirror.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/entry.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/0.1/runtime/entry.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/wrappers/CodeEditor.tsx",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/codemirror/0.1/wrappers/CodeEditor.tsx"
        ),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/0.1/keywords.json"),
    },
    // The bundle is edited by hand (no build step); the canvas notes live in it.
    // cargo-watch does not watch blessed/, so a change there needs a src edit
    // beside it to rebuild the dev server.
    EmbeddedAsset {
        path: "zeb/graphui/0.1/runtime/graphui.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/graphui/0.1/runtime/graphui.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/wrappers/GraphCanvas.tsx",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/graphui/0.1/wrappers/GraphCanvas.tsx"
        ),
    },
    EmbeddedAsset {
        path: "zeb/threejs/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/r183/bundle.min.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/r183/bundle.min.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/runtime/threejs.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/threejs/0.1/runtime/threejs.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/wrappers/ThreeScene.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/0.1/wrappers/ThreeScene.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/wrappers/VrmViewer.tsx",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/threejs-vrm/0.1/wrappers/VrmViewer.tsx"
        ),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/runtime/markdown.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/markdown/0.1/runtime/markdown.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/wrappers/Markdown.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/0.1/wrappers/Markdown.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/runtime/use.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/0.1/runtime/use.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/runtime/livegeo.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/livegeo/0.1/runtime/livegeo.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs",
        bytes: include_bytes!(
            "../../../blessed/rwe-libraries/prosemirror/0.1/runtime/prosemirror.bundle.mjs"
        ),
    },
    // No library.json for zeb/react. The other entries here describe packages a
    // project installs; the engine is not one, and a manifest beside these would
    // read as though it were.
    EmbeddedAsset {
        path: "zeb/react/0.1/runtime/zeb_react.js",
        bytes: include_bytes!("../../rwe/runtime/zeb_react.js"),
    },
    EmbeddedAsset {
        path: "zeb/react/0.1/runtime/zeb_react.mjs",
        bytes: include_bytes!("../../rwe/runtime/zeb_react.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/pdf/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/pdf/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/runtime/pdf.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/pdf/0.1/runtime/pdf.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/d3/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/pdf/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/package.yaml"),
    },
    EmbeddedAsset {
        path: "zeb/use/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/package.yaml"),
    },
    // Notices are package content: seeding enumerates this table, not the
    // filesystem. A declared notice must travel with the installed package.
    EmbeddedAsset {
        path: "zeb/codemirror/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/codemirror/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/d3/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/d3/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/MODIFICATIONS",
        bytes: include_bytes!("../../../blessed/rwe-libraries/deckgl/MODIFICATIONS"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/graphui/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/livegeo/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/LICENSE.marked",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/LICENSE.marked"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/LICENSE.dompurify",
        bytes: include_bytes!("../../../blessed/rwe-libraries/markdown/LICENSE.dompurify"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/pdf/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/prosemirror/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/threejs-vrm/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/use/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/use/LICENSE"),
    },
];

/// Bytes for the Studio's own pages.
///
/// Reads the platform's vendored copies first, and only falls back to the hub
/// catalogue for a library the platform does not vendor. The fallback exists so
/// a page reaching for something outside the vendored five still answers rather
/// than 404ing; it is not the path the Studio is expected to take.
pub fn platform_library_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_VENDOR_ASSETS
        .iter()
        .chain(PLATFORM_LIBRARY_ASSETS.iter())
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

/// Bytes for a project that has not installed the library itself.
///
/// Always the hub catalogue: an un-installed project should see the version the
/// hub would have given it, not whichever copy the Studio happens to pin.
pub fn hub_catalogue_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_LIBRARY_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

pub fn platform_public_asset(path: &str) -> Option<&'static [u8]> {
    match path.trim_start_matches('/').replace('\\', "/").as_str() {
        "branding/logo.svg" => Some(BRAND_LOGO_SVG),
        "branding/logo.png" => Some(BRAND_LOGO_PNG),
        "branding/favicon.svg" => Some(BRAND_FAVICON_SVG),
        "branding/favicon.ico" => Some(BRAND_FAVICON_ICO),
        "branding/favicon-16.png" => Some(BRAND_FAVICON_16_PNG),
        "branding/favicon-32.png" => Some(BRAND_FAVICON_32_PNG),
        "branding/apple-touch-icon.png" => Some(BRAND_APPLE_TOUCH_ICON_PNG),
        "platform/main.css" => Some(PLATFORM_MAIN_CSS.as_bytes()),
        "platform/db-suite.css" => Some(PLATFORM_DB_SUITE_CSS.as_bytes()),
        "platform/db-connections.css" => Some(PLATFORM_DB_CONNECTIONS_CSS.as_bytes()),
        "platform/devicons.css" => Some(PLATFORM_DEVICONS_CSS.as_bytes()),
        "platform/fonts/HankenGrotesk-cyrillic-ext.woff2" => Some(FONT_HANKENGROTESK_CYRILLIC_EXT_WOFF2),
        "platform/fonts/HankenGrotesk-latin-ext.woff2" => Some(FONT_HANKENGROTESK_LATIN_EXT_WOFF2),
        "platform/fonts/HankenGrotesk-latin.woff2" => Some(FONT_HANKENGROTESK_LATIN_WOFF2),
        "platform/fonts/HankenGrotesk-vietnamese.woff2" => Some(FONT_HANKENGROTESK_VIETNAMESE_WOFF2),
        "platform/fonts/JetBrainsMono-cyrillic-ext.woff2" => Some(FONT_JETBRAINSMONO_CYRILLIC_EXT_WOFF2),
        "platform/fonts/JetBrainsMono-cyrillic.woff2" => Some(FONT_JETBRAINSMONO_CYRILLIC_WOFF2),
        "platform/fonts/JetBrainsMono-greek.woff2" => Some(FONT_JETBRAINSMONO_GREEK_WOFF2),
        "platform/fonts/JetBrainsMono-latin-ext.woff2" => Some(FONT_JETBRAINSMONO_LATIN_EXT_WOFF2),
        "platform/fonts/JetBrainsMono-latin.woff2" => Some(FONT_JETBRAINSMONO_LATIN_WOFF2),
        "platform/fonts/JetBrainsMono-vietnamese.woff2" => Some(FONT_JETBRAINSMONO_VIETNAMESE_WOFF2),
        "platform/fonts/SpaceGrotesk-latin-ext.woff2" => Some(FONT_SPACEGROTESK_LATIN_EXT_WOFF2),
        "platform/fonts/SpaceGrotesk-latin.woff2" => Some(FONT_SPACEGROTESK_LATIN_WOFF2),
        "platform/fonts/SpaceGrotesk-vietnamese.woff2" => Some(FONT_SPACEGROTESK_VIETNAMESE_WOFF2),
        _ => None,
    }
}

#[cfg(test)]
mod vendor_tests {
    use super::*;

    /// An icon file that is on disk but not in `PLATFORM_NODE_ICON_ASSETS` is
    /// invisible to every editor, and nothing said so for a whole afternoon.
    /// The table is hand-written, so this keeps it honest both ways.
    #[test]
    fn every_node_icon_on_disk_is_embedded_and_every_embedded_icon_exists() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/assets/node-icons/zebflow");
        let on_disk: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
            .expect("icons dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".svg"))
            .map(|n| format!("zebflow/{n}"))
            .collect();
        // The manifest sits beside the icons in the same table.
        let embedded: std::collections::BTreeSet<String> =
            PLATFORM_NODE_ICON_ASSETS.iter().map(|a| a.path.to_string()).filter(|p| p.ends_with(".svg")).collect();
        let missing: Vec<_> = on_disk.difference(&embedded).collect();
        let stale: Vec<_> = embedded.difference(&on_disk).collect();
        assert!(missing.is_empty(), "icons on disk not embedded — add them to PLATFORM_NODE_ICON_ASSETS: {missing:?}");
        assert!(stale.is_empty(), "embedded icons with no file: {stale:?}");
    }

    /// The Studio vendors every library its own pages import.
    ///
    /// If a page reaches for one that is not vendored, the lookup silently
    /// falls back to the hub catalogue and the coupling is back — quietly, and
    /// only for that library.
    #[test]
    fn the_studio_vendors_every_library_its_pages_import() {
        let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/src/platform/web/templates");
        let mut imported = std::collections::BTreeSet::new();
        collect_zeb_imports(std::path::Path::new(templates), &mut imported);

        for library in &imported {
            // `zeb/ui/*` is source the compiler inlines from this build's own
            // `blessed/source-libraries/`, not a hub bundle: the gallery at
            // /dev/design-system/ui imports it on purpose, to show exactly what
            // a project page gets from this binary. It moves with the build,
            // never with the hub.
            if library.starts_with("zeb/ui/") {
                continue;
            }
            let prefix = format!("{library}/");
            let vendored = PLATFORM_VENDOR_ASSETS
                .iter()
                .any(|asset| asset.path.starts_with(&prefix));
            assert!(
                vendored,
                "the Studio imports '{library}' but does not vendor it, so it would \
                 fall back to the hub catalogue and move whenever the hub does"
            );
        }
        assert!(
            imported.len() >= 3,
            "only {} zeb libraries found in templates — the scan stopped matching",
            imported.len()
        );
    }

    /// Nothing the Studio serves is read out of the hub's directory.
    #[test]
    fn no_vendored_byte_is_read_from_the_blessed_tree() {
        let source = include_str!("embedded.rs");
        let start = source
            .find("pub const PLATFORM_VENDOR_ASSETS")
            .expect("the vendor list exists");
        let end = source[start..].find("\n];").expect("the vendor list ends") + start;
        let list = &source[start..end];
        assert!(
            !list.contains("blessed/"),
            "the vendor list reaches into blessed/, which is the coupling it exists to remove"
        );
    }

    fn collect_zeb_imports(dir: &std::path::Path, out: &mut std::collections::BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_zeb_imports(&path, out);
                continue;
            }
            let is_source = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "tsx" | "ts"));
            if !is_source {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (index, _) in text.match_indices("from \"zeb/") {
                let rest = &text[index + "from \"".len()..];
                let name: String = rest.chars().take_while(|c| *c != '"').collect();
                // zeb/react is the engine, served from the platform's own runtime.
                if name != "zeb/react" && !name.is_empty() {
                    out.insert(name);
                }
            }
        }
    }
}
