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
        bytes: include_bytes!(
            "../../../libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs"
        ),
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
        bytes: include_bytes!(
            "../../../libraries/zeb/prosemirror/0.1/wrappers/ProseEditor.tsx"
        ),
    },
];

pub fn platform_library_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_LIBRARY_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}
