//! Embedded platform templates and official library assets.

/// One embedded file shipped inside the binary.
pub struct EmbeddedAsset {
    pub path: &'static str,
    pub bytes: &'static [u8],
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
        path: "zeb/d3/0.1/wrappers/D3Bars.tsx",
        bytes: include_bytes!("../../../libraries/zeb/d3/0.1/wrappers/D3Bars.tsx"),
    },
    EmbeddedAsset {
        path: "zeb/devicons/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/devicons/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/devicons/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/devicons/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/devicons/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/devicons/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/devicons/0.1/runtime/devicons.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/devicons/0.1/runtime/devicons.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/devicons/0.1/runtime/devicons.css",
        bytes: include_bytes!("../../../libraries/zeb/devicons/0.1/runtime/devicons.css"),
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
        path: "zeb/interact/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/interact/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/interact/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/interact/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/interact/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/interact/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/interact/0.1/runtime/interact.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/interact/0.1/runtime/interact.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/stateutil/0.1/library.json",
        bytes: include_bytes!("../../../libraries/zeb/stateutil/0.1/library.json"),
    },
    EmbeddedAsset {
        path: "zeb/stateutil/0.1/exports.json",
        bytes: include_bytes!("../../../libraries/zeb/stateutil/0.1/exports.json"),
    },
    EmbeddedAsset {
        path: "zeb/stateutil/0.1/keywords.json",
        bytes: include_bytes!("../../../libraries/zeb/stateutil/0.1/keywords.json"),
    },
    EmbeddedAsset {
        path: "zeb/stateutil/0.1/runtime/stateutil.bundle.mjs",
        bytes: include_bytes!("../../../libraries/zeb/stateutil/0.1/runtime/stateutil.bundle.mjs"),
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
        path: "zeb/threejs/0.1/runtime/threejs.global.js",
        bytes: include_bytes!("../../../libraries/zeb/threejs/0.1/runtime/threejs.global.js"),
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
        bytes: include_bytes!(
            "../../../libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs"
        ),
    },
    EmbeddedAsset {
        path: "zeb/markdown/0.1/wrappers/Markdown.tsx",
        bytes: include_bytes!("../../../libraries/zeb/markdown/0.1/wrappers/Markdown.tsx"),
    },
];

pub fn platform_library_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_LIBRARY_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}
