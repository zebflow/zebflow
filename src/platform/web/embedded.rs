//! Embedded platform templates and official library assets.

/// One embedded file shipped inside the binary.
pub struct EmbeddedAsset {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

const BRAND_LOGO_SVG: &[u8] = include_bytes!("assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("assets/branding/logo.png");
const PLATFORM_MAIN_CSS: &str = concat!(
    include_str!("templates/styles/main.css"),
    "\n\n",
    include_str!("templates/pages/project-studio/styles.css"),
);
const PLATFORM_DB_SUITE_CSS: &str = include_str!("templates/styles/db-suite.css");
const PLATFORM_DB_CONNECTIONS_CSS: &str = include_str!("templates/styles/db-connections.css");

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
        path: "zebflow/n.fs.thumbnail.svg",
        bytes: include_bytes!("assets/node-icons/zebflow/n.fs.thumbnail.svg"),
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
];

pub fn platform_node_icon_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_NODE_ICON_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}

/// Embedded official composite node packages.
///
/// Each entry uses path format: `{slug}/node.json`, `{slug}/pipeline.zf.json`, `{slug}/icon.svg`.
/// To add an official composite node, create the package under `composites/{slug}/` and add
/// `include_bytes!()` entries here.
pub const PLATFORM_COMPOSITE_NODE_ASSETS: &[EmbeddedAsset] = &[
    // ── Telegram (multi-node package) ───────────────────────────────────
    EmbeddedAsset { path: "telegram/definition.json", bytes: include_bytes!("../../../composites/telegram/definition.json") },
    EmbeddedAsset { path: "telegram/icon.svg", bytes: include_bytes!("../../../composites/telegram/icon.svg") },
    EmbeddedAsset { path: "telegram/icons/trigger.svg", bytes: include_bytes!("../../../composites/telegram/icons/trigger.svg") },
    EmbeddedAsset { path: "telegram/icons/send.svg", bytes: include_bytes!("../../../composites/telegram/icons/send.svg") },
    EmbeddedAsset { path: "telegram/icons/send-photo.svg", bytes: include_bytes!("../../../composites/telegram/icons/send-photo.svg") },
    EmbeddedAsset { path: "telegram/icons/send-document.svg", bytes: include_bytes!("../../../composites/telegram/icons/send-document.svg") },
    EmbeddedAsset { path: "telegram/icons/edit.svg", bytes: include_bytes!("../../../composites/telegram/icons/edit.svg") },
    EmbeddedAsset { path: "telegram/functions/send-message.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/send-message.zf.json") },
    EmbeddedAsset { path: "telegram/functions/send-photo.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/send-photo.zf.json") },
    EmbeddedAsset { path: "telegram/functions/send-document.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/send-document.zf.json") },
    EmbeddedAsset { path: "telegram/functions/edit-message.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/edit-message.zf.json") },
    EmbeddedAsset { path: "telegram/functions/register-webhook.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/register-webhook.zf.json") },
    EmbeddedAsset { path: "telegram/functions/delete-webhook.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/delete-webhook.zf.json") },
    EmbeddedAsset { path: "telegram/functions/transform-update.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/transform-update.zf.json") },
    // ── OpenAI Embedding ────────────────────────────────────────────────
    EmbeddedAsset { path: "openai-embedding/definition.json", bytes: include_bytes!("../../../composites/openai-embedding/definition.json") },
    EmbeddedAsset { path: "openai-embedding/icon.svg", bytes: include_bytes!("../../../composites/openai-embedding/icon.svg") },
    EmbeddedAsset { path: "openai-embedding/icons/embedding.svg", bytes: include_bytes!("../../../composites/openai-embedding/icons/embedding.svg") },
    EmbeddedAsset { path: "openai-embedding/functions/embed.zf.json", bytes: include_bytes!("../../../composites/openai-embedding/functions/embed.zf.json") },
];

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

pub const PLATFORM_LIBRARY_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "zeb/d3/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/d3/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/d3/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/d3/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/0.1/runtime/d3.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/d3/0.1/runtime/d3.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/runtime/deckgl.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/runtime/deckgl.patched.mjs",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/runtime/deckgl.patched.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/0.1/wrappers/DeckMap.tsx",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/0.1/wrappers/DeckMap.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
        bytes: include_bytes!(
            "../../../libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/runtime/entry.mjs",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/runtime/entry.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/wrappers/CodeEditor.tsx",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/wrappers/CodeEditor.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/graphui/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/graphui/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/graphui/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/runtime/graphui.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/0.1/wrappers/GraphCanvas.tsx",
        bytes: include_bytes!("../../../libraries/zeb/graphui/0.1/wrappers/GraphCanvas.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/d3/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/d3/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/deckgl/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/deckgl/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/graphui/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/graphui/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/icons/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/icons/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/markdown/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs-vrm/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/use/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/livegeo/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/r183/bundle.min.mjs",
        bytes: include_bytes!("../../../libraries/zeb/threejs/r183/bundle.min.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/runtime/threejs.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/runtime/threejs.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/threejs/0.1/wrappers/ThreeScene.tsx",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/wrappers/ThreeScene.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs-vrm/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs-vrm/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/threejs-vrm/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs",
        bytes: include_bytes!(
            "../../../libraries/zeb/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/threejs-vrm/0.1/wrappers/VrmViewer.tsx",
        bytes: include_bytes!("../../../libraries/zeb/threejs-vrm/0.1/wrappers/VrmViewer.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/runtime/markdown.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/wrappers/Markdown.tsx",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/wrappers/Markdown.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/use/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/use/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/use/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/use/0.1/runtime/use.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/use/0.1/runtime/use.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/livegeo/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/livegeo/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/livegeo/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/livegeo/0.1/runtime/livegeo.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/livegeo/0.1/runtime/livegeo.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/icons/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/icons/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/icons/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/icons/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/icons/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/icons/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/icons/0.1/runtime/icons.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/icons/0.1/runtime/icons.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/icons/0.1/runtime/devicons.css",
        bytes: include_bytes!("../../../libraries/zeb/icons/0.1/runtime/devicons.css"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/prosemirror/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/prosemirror/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/prosemirror/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/prosemirror/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs",
        bytes: include_bytes!(
            "../../../libraries/zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/prosemirror/0.1/wrappers/ProseEditor.tsx",
        bytes: include_bytes!("../../../libraries/zeb/prosemirror/0.1/wrappers/ProseEditor.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/preact/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/preact/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/preact/0.1/runtime/preact.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/preact/0.1/runtime/preact.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/manifest.json",
        bytes: include_bytes!("../../../libraries/zeb/pdf/manifest.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/pdf/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/pdf/0.1/runtime/pdf.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/pdf/0.1/runtime/pdf.bundle.mjs"),
    },
];

pub fn platform_library_asset(path: &str) -> Option<&'static [u8]> {
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
        "platform/main.css" => Some(PLATFORM_MAIN_CSS.as_bytes()),
        "platform/db-suite.css" => Some(PLATFORM_DB_SUITE_CSS.as_bytes()),
        "platform/db-connections.css" => Some(PLATFORM_DB_CONNECTIONS_CSS.as_bytes()),
        _ => None,
    }
}
