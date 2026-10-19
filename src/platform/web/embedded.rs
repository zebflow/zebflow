//! Embedded platform templates and official library assets.
//!
//! `blessed/` is not a watched path, so a hand-edited library bundle (the
//! graphui canvas, for one) only reaches a running dev server when something
//! under `src/` changes too. (Touched 2026-09-20: note colour picker, resizable
//! input widgets, undeclared `:error` pins kept through a save. Touched
//! 2026-09-21: `retry` / `error_routed` run badges — an orange ring with the
//! attempt count, never red. Touched 2026-10-04: `skip` is `empty`, and
//! `skipped` — a node on a branch not taken — is a hollow grey ring.)

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
    // zeb/world, pinned for the Studio's own maps (build/build.mjs --vendor rewrites this list): the
    // package, its lazy chunks and tile worker next to the entry. earth-lite is served from the hub's copy.
    EmbeddedAsset {
        path: "zeb/world/0.1/exports.json",
        bytes: include_bytes!("vendor/zeb/world/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/keywords.json",
        bytes: include_bytes!("vendor/zeb/world/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/library.json",
        bytes: include_bytes!("vendor/zeb/world/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/forms-Caslj2eP.mjs",
        bytes: include_bytes!("vendor/zeb/world/0.1/runtime/forms-Caslj2eP.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/solids-BvjrieSI.mjs",
        bytes: include_bytes!("vendor/zeb/world/0.1/runtime/solids-BvjrieSI.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/tiles.worker.mjs",
        bytes: include_bytes!("vendor/zeb/world/0.1/runtime/tiles.worker.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/world.bundle.mjs",
        bytes: include_bytes!("vendor/zeb/world/0.1/runtime/world.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/zone-places-B7wsdUjy.mjs",
        bytes: include_bytes!("vendor/zeb/world/0.1/runtime/zone-places-B7wsdUjy.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/DATA-LICENCES.md",
        bytes: include_bytes!("vendor/zeb/world/DATA-LICENCES.md"),
    },
    EmbeddedAsset {
        path: "zeb/world/FILES-SHA256.txt",
        bytes: include_bytes!("vendor/zeb/world/FILES-SHA256.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/LICENSE",
        bytes: include_bytes!("vendor/zeb/world/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/world/NOTICE",
        bytes: include_bytes!("vendor/zeb/world/NOTICE"),
    },
    EmbeddedAsset {
        path: "zeb/world/README.md",
        bytes: include_bytes!("vendor/zeb/world/README.md"),
    },
    EmbeddedAsset {
        path: "zeb/world/THIRD-PARTY-NOTICES.txt",
        bytes: include_bytes!("vendor/zeb/world/THIRD-PARTY-NOTICES.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/manifest.json",
        bytes: include_bytes!("vendor/zeb/world/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/package.yaml",
        bytes: include_bytes!("vendor/zeb/world/package.yaml"),
    },
];

pub const PLATFORM_NODE_ICON_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "manifest.json",
        bytes: include_bytes!("assets/node-icons/manifest.json"),
    },
    EmbeddedAsset {
        path: "zebflow/ai.text.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ai.text.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ai.audio.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ai.audio.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/auth.oauth.approve.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/auth.oauth.approve.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/auth.token.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/auth.token.create.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/browser.page.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/browser.page.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/crypto.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/crypto.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.archive.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.archive.create.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.copy.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.copy.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.archive.extract.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.archive.extract.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.delete.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.delete.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.head.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.head.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.folder.list.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.folder.list.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.folder.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.folder.create.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.move.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.move.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.pdf.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.pdf.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.file.put.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.file.put.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.image.chromakey.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.image.chromakey.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.image.thumbnail.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.image.thumbnail.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/fs.image.render.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/fs.image.render.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/function.result.call.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/function.result.call.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/geo.dataset.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/geo.dataset.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/geo.dataset.inspect.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/geo.dataset.inspect.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/http.response.fetch.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/http.response.fetch.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.collect.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.collect.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.foreach.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.foreach.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.if.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.if.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.match.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.match.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.reduce.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.reduce.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.retry.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.retry.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.put.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.put.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.delete.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.delete.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.head.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.head.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.expire.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.expire.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.entry.increment.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.entry.increment.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/kv.message.publish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/kv.message.publish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/postgres.query.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/postgres.query.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/sekejap.query.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/sekejap.query.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/table.data.convert.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/table.data.convert.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/table.query.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/table.query.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/javascript.script.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/javascript.script.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/sqlite.query.run.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/sqlite.query.run.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.function.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.function.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.manual.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.manual.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.mcp.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.mcp.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/mapserver.layer.publish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/mapserver.layer.publish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/mapserver.layer.unpublish.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/mapserver.layer.unpublish.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/mapserver.layer.get.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/mapserver.layer.get.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/mapserver.layer.list.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/mapserver.layer.list.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.topic.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.topic.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.schedule.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.schedule.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.webhook.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.webhook.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.error.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.error.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.room.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.room.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/trigger.socket.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/trigger.socket.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/web.response.send.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/web.response.send.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/web.site.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/web.site.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.message.send.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.message.send.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.state.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.state.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.room.open.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.room.open.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.room.close.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.room.close.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.room.list.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.room.list.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.ticket.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.ticket.create.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.session.close.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.session.close.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ws.session.tag.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ws.session.tag.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/ai.embedding.generate.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/ai.embedding.generate.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/auth.token.verify.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/auth.token.verify.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/logic.concept.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/logic.concept.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/mail.message.send.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/mail.message.send.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.text.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.text.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.number.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.number.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.boolean.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.boolean.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.json.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.json.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.file.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.file.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.files.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.files.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.image.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.image.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.audio.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.audio.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/input.video.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/input.video.svg"),
    },
    EmbeddedAsset {
        path: "zebflow/sekejap.record.create.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/sekejap.record.create.svg"),
    },
];

pub fn platform_node_icon_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_NODE_ICON_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

/// The icon of a native node kind: `zebflow/<kind>.svg`, or the icon whose
/// manifest entry lists the kind in `supports` — one svg serving a family
/// (`crypto.svg` for every `crypto.*` kind).
pub fn platform_node_icon_for_kind(kind: &str) -> Option<&'static [u8]> {
    static SUPPORTED: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
    if let Some(bytes) = platform_node_icon_asset(&format!("zebflow/{kind}.svg")) {
        return Some(bytes);
    }
    let supported = SUPPORTED.get_or_init(|| {
        let manifest: serde_json::Value = platform_node_icon_asset("manifest.json")
            .and_then(|bytes| serde_json::from_slice(bytes).ok())
            .unwrap_or_default();
        let mut map = std::collections::HashMap::new();
        for entry in manifest["icons"].as_object().into_iter().flat_map(|icons| icons.values()) {
            let Some(path) = entry["path"].as_str().and_then(|p| p.strip_prefix("/assets/node-icons/")) else { continue };
            for supported in entry["supports"].as_array().into_iter().flatten().filter_map(|k| k.as_str()) {
                map.insert(supported.to_string(), path.to_string());
            }
        }
        map
    });
    supported.get(kind).and_then(|path| platform_node_icon_asset(path))
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
        path: "zeb/rooms/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/manifest.json"),
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
        path: "zeb/rooms/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/rooms/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/rooms/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/rooms/0.1/runtime/rooms.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/0.1/runtime/rooms.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/rooms/README.md",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/README.md"),
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
    // zeb/potoru: every file of the package. The entry finds its lazy chunks,
    // wasm engines, score worker and fonts with `new URL(…, import.meta.url)`,
    // so each one has to answer next to it under `0.1/runtime/`. Chunk names
    // carry content hashes: a rebuild of the library rewrites this list.
    EmbeddedAsset {
        path: "zeb/potoru/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/action-core-TOJAEIPU.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/action-core-TOJAEIPU.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/canvas-effects-UNL5IA6Q.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/canvas-effects-UNL5IA6Q.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/canvas-effects-registry-CIRAQN3P.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/canvas-effects-registry-CIRAQN3P.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-5MJDBQ6D.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-5MJDBQ6D.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-6BUSNEOM.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-6BUSNEOM.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-ACMIY2U6.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-ACMIY2U6.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-AUFJMJOH.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-AUFJMJOH.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-AWXDNTLF.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-AWXDNTLF.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-BNXQLZB7.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-BNXQLZB7.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-CCJSFTNP.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-CCJSFTNP.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-EQBLUIJT.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-EQBLUIJT.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-GAWYRDBH.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-GAWYRDBH.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-GDLLO7MM.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-GDLLO7MM.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-H5EAQMXY.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-H5EAQMXY.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-KPAZEOOS.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-KPAZEOOS.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-LYSIUW53.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-LYSIUW53.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-MULTMAJ7.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-MULTMAJ7.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-NNRPRQL6.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-NNRPRQL6.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-O26L46OV.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-O26L46OV.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-P63CGZHL.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-P63CGZHL.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-RVLRL5NF.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-RVLRL5NF.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-XMLOGUJ5.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-XMLOGUJ5.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-XP3QTEDN.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-XP3QTEDN.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/chunk-Y4OXOE4X.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/chunk-Y4OXOE4X.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/compact-container-JTYLI2SI.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/compact-container-JTYLI2SI.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/compiler-SX65UBAB.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/compiler-SX65UBAB.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/container-GH5VOGFO.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/container-GH5VOGFO.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/entry.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/entry.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/folder-fs-3BBTANXU.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/folder-fs-3BBTANXU.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/font-aliases-QNVLCUAQ.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/font-aliases-QNVLCUAQ.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/SOURCES.md",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/SOURCES.md"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-font-OFL.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-font-OFL.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-font.woff2",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-font.woff2"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-mono-OFL.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-mono-OFL.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-mono.woff2",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-mono.woff2"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-serif-OFL.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-serif-OFL.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/fonts/basic-serif.woff2",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/fonts/basic-serif.woff2"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/g2p-wasm-DI6V376L.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/g2p-wasm-DI6V376L.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/generator-registry-UOO4S5G4.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/generator-registry-UOO4S5G4.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/generators-NZXCU6XN.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/generators-NZXCU6XN.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/library-2S7I3L5U.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/library-2S7I3L5U.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/node_fs-ZYXC4DSI.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/node_fs-ZYXC4DSI.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/node_path-BW23E7JS.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/node_path-BW23E7JS.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/package-container-PAD3WPI4.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/package-container-PAD3WPI4.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/package.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/package.json"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/path-core-loader-SLKY4XXY.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/path-core-loader-SLKY4XXY.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/player-5SBRV3VA.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/player-5SBRV3VA.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru-authoring-sandbox.js",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru-authoring-sandbox.js"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru_action_wasm_bg.wasm",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru_action_wasm_bg.wasm"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru_g2p_wasm_bg.wasm",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru_g2p_wasm_bg.wasm"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru_render_wasm_bg.wasm",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru_render_wasm_bg.wasm"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/potoru_score_wasm_bg.wasm",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/potoru_score_wasm_bg.wasm"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/resolver-PX5GWGRR.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/resolver-PX5GWGRR.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/runtime-tick-YXO36JNL.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/runtime-tick-YXO36JNL.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/score-wasm-HMOAUMLF.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/score-wasm-HMOAUMLF.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/score-worker.js",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/score-worker.js"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/snapshot-QBKH6MPC.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/snapshot-QBKH6MPC.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/compiler.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/compiler.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/patch.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/patch.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/player.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/player.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/sandbox-api.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/sandbox-api.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/snapshot.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/snapshot.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/src/widget.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/src/widget.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/svg-surface-U3RQ262G.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/svg-surface-U3RQ262G.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/voices-M6MJNPUQ.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/voices-M6MJNPUQ.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/runtime/voices-panel-3RBSS4U3.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/runtime/voices-panel-3RBSS4U3.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/wrappers/PotoEditor.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/wrappers/PotoEditor.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/wrappers/PotoPlayer.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/wrappers/PotoPlayer.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/0.1/wrappers/PotoSnapshot.tsx",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/0.1/wrappers/PotoSnapshot.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/FILES-SHA256.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/FILES-SHA256.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/NOTICE.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/NOTICE.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/README.md",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/README.md"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/THIRD-PARTY-NOTICES.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/THIRD-PARTY-NOTICES.txt"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/potoru/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/potoru/package.yaml"),
    },
    // zeb/world: every file of the package. The entry finds its lazy chunks and the tile
    // worker with `new URL(…, import.meta.url)`, so each one has to answer next to it under
    // `0.1/runtime/`. Chunk names carry content hashes: a rebuild of the library rewrites this list.
    EmbeddedAsset {
        path: "zeb/world/0.1/exports.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/keywords.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/library.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/forms-Caslj2eP.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/runtime/forms-Caslj2eP.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/solids-BvjrieSI.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/runtime/solids-BvjrieSI.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/tiles.worker.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/runtime/tiles.worker.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/world.bundle.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/runtime/world.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/0.1/runtime/zone-places-B7wsdUjy.mjs",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/0.1/runtime/zone-places-B7wsdUjy.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/world/DATA-LICENCES.md",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/DATA-LICENCES.md"),
    },
    EmbeddedAsset {
        path: "zeb/world/FILES-SHA256.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/FILES-SHA256.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/LICENSE"),
    },
    EmbeddedAsset {
        path: "zeb/world/NOTICE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/NOTICE"),
    },
    EmbeddedAsset {
        path: "zeb/world/README.md",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/README.md"),
    },
    EmbeddedAsset {
        path: "zeb/world/THIRD-PARTY-NOTICES.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/THIRD-PARTY-NOTICES.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/0/0/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/0/0/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/1/0/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/1/0/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/1/0/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/1/0/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/1/1/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/1/1/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/1/1/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/1/1/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/0/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/0/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/0/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/0/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/0/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/0/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/0/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/0/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/1/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/1/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/1/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/1/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/1/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/1/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/1/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/1/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/2/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/2/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/2/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/2/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/2/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/2/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/2/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/2/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/3/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/3/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/3/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/3/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/3/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/3/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/2/3/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/2/3/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/0/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/0/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/1/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/1/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/2/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/2/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/3/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/3/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/4/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/4/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/5/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/5/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/6/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/6/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/3/7/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/3/7/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/0/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/0/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/1/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/1/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/10/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/10/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/11/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/11/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/12/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/12/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/13/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/13/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/14/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/14/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/15/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/15/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/2/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/2/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/3/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/3/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/4/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/4/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/5/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/5/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/6/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/6/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/7/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/7/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/8/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/8/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/0.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/0.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/1.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/1.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/10.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/10.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/11.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/11.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/12.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/12.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/13.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/13.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/14.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/14.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/15.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/15.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/2.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/2.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/3.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/3.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/4.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/4.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/5.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/5.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/6.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/6.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/7.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/7.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/8.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/8.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/4/9/9.jpg",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/4/9/9.jpg"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/ATTRIBUTION.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/ATTRIBUTION.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/earth-lite/LICENCE.txt",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/earth-lite/LICENCE.txt"),
    },
    EmbeddedAsset {
        path: "zeb/world/manifest.json",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/world/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/world/package.yaml"),
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
    EmbeddedAsset {
        path: "zeb/rooms/package.yaml",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/package.yaml"),
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
    EmbeddedAsset {
        path: "zeb/rooms/LICENSE",
        bytes: include_bytes!("../../../blessed/rwe-libraries/rooms/LICENSE"),
    },
];

/// Bytes for the Studio's own pages.
///
/// Reads the platform's vendored copies first, and only falls back to the hub
/// catalogue for a library the platform does not vendor. The fallback exists so
/// a page reaching for something outside the vendored set still answers rather
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

    /// A family icon serves every kind its manifest entry lists.
    #[test]
    fn a_kind_finds_the_icon_its_manifest_entry_supports() {
        let crypto = platform_node_icon_asset("zebflow/crypto.svg").expect("crypto icon");
        for kind in crate::pipeline::nodes::basic::crypto::KINDS {
            assert_eq!(platform_node_icon_for_kind(kind), Some(crypto), "{kind}");
        }
        assert!(platform_node_icon_for_kind("logic.if").is_some());
        assert!(platform_node_icon_for_kind("crypto.nothing.here").is_none());
    }

    /// Every room node draws the room frame `trigger.room` draws and differs
    /// only in its badge, so the family reads as one in the editor. Each icon
    /// is small and carries no editor metadata, which is where a name or a
    /// path would hide.
    #[test]
    fn every_room_node_has_a_small_icon_with_the_room_frame() {
        fn base_object(svg: &str) -> &str {
            let start = svg.find("<g id=\"base-object\"").expect("base-object group");
            let end = start + svg[start..].find("</g>").expect("base-object end");
            &svg[start..end]
        }
        let room = std::str::from_utf8(platform_node_icon_for_kind("trigger.room").expect("trigger.room icon")).unwrap();
        let kinds = [
            "ws.room.open",
            "ws.room.close",
            "ws.room.list",
            "ws.ticket.create",
            "ws.session.close",
            "ws.session.tag",
        ];
        for kind in kinds {
            let bytes = platform_node_icon_for_kind(kind).unwrap_or_else(|| panic!("{kind} has no icon"));
            let svg = std::str::from_utf8(bytes).unwrap();
            assert!(bytes.len() < 2048, "{kind} icon is {} bytes", bytes.len());
            assert!(svg.contains("viewBox=\"0 0 128 128\""), "{kind}");
            assert_eq!(base_object(svg), base_object(room), "{kind} redraws the room frame");
            assert!(svg.contains("<g id=\"operation-badge\""), "{kind} has no badge");
            for banned in ["<metadata", "inkscape", "sodipodi", "<title", "/Users/", "/home/"] {
                assert!(!svg.contains(banned), "{kind} icon carries {banned}");
            }
        }
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
