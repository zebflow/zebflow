//! Embedded platform templates and official library assets.

/// One embedded file shipped inside the binary.
pub struct EmbeddedAsset {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

pub const PLATFORM_TEMPLATE_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        path: "components/layout/project-studio-shell.tsx",
        bytes: include_bytes!("templates/components/layout/project-studio-shell.tsx"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-shell.ts",
        bytes: include_bytes!("templates/components/behavior/project-shell.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/pipeline-editor.ts",
        bytes: include_bytes!("templates/components/behavior/pipeline-editor.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-credentials.ts",
        bytes: include_bytes!("templates/components/behavior/project-credentials.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-db-connections.ts",
        bytes: include_bytes!("templates/components/behavior/project-db-connections.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-db-suite.ts",
        bytes: include_bytes!("templates/components/behavior/project-db-suite.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-db-suite-postgresql.ts",
        bytes: include_bytes!("templates/components/behavior/project-db-suite-postgresql.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-db-suite-sjtable.ts",
        bytes: include_bytes!("templates/components/behavior/project-db-suite-sjtable.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-settings.ts",
        bytes: include_bytes!("templates/components/behavior/project-settings.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/template-editor.ts",
        bytes: include_bytes!("templates/components/behavior/template-editor.ts"),
    },
    EmbeddedAsset {
        path: "components/behavior/project-docs.ts",
        bytes: include_bytes!("templates/components/behavior/project-docs.ts"),
    },
    EmbeddedAsset {
        path: "components/platform-sidebar.tsx",
        bytes: include_bytes!("templates/components/platform-sidebar.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/markdown.tsx",
        bytes: include_bytes!("templates/components/ui/markdown.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/button.tsx",
        bytes: include_bytes!("templates/components/ui/button.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/card.tsx",
        bytes: include_bytes!("templates/components/ui/card.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/card-header.tsx",
        bytes: include_bytes!("templates/components/ui/card-header.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/card-title.tsx",
        bytes: include_bytes!("templates/components/ui/card-title.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/card-description.tsx",
        bytes: include_bytes!("templates/components/ui/card-description.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/card-content.tsx",
        bytes: include_bytes!("templates/components/ui/card-content.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/field.tsx",
        bytes: include_bytes!("templates/components/ui/field.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/input.tsx",
        bytes: include_bytes!("templates/components/ui/input.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/label.tsx",
        bytes: include_bytes!("templates/components/ui/label.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/sonner.tsx",
        bytes: include_bytes!("templates/components/ui/sonner.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/hierarchy-tree.tsx",
        bytes: include_bytes!("templates/components/ui/hierarchy-tree.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/template-folder-tree.tsx",
        bytes: include_bytes!("templates/components/ui/template-folder-tree.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/webhook-route-tree.tsx",
        bytes: include_bytes!("templates/components/ui/webhook-route-tree.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-home.tsx",
        bytes: include_bytes!("templates/pages/platform-home.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-login.tsx",
        bytes: include_bytes!("templates/pages/platform-login.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-build-templates.tsx",
        bytes: include_bytes!("templates/pages/platform-project-build-templates.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-docs.tsx",
        bytes: include_bytes!("templates/pages/platform-project-docs.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-credentials.tsx",
        bytes: include_bytes!("templates/pages/platform-project-credentials.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-pipelines.tsx",
        bytes: include_bytes!("templates/pages/platform-project-pipelines.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-pipelines-registry.tsx",
        bytes: include_bytes!("templates/pages/platform-project-pipelines-registry.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-section.tsx",
        bytes: include_bytes!("templates/pages/platform-project-section.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-settings.tsx",
        bytes: include_bytes!("templates/pages/platform-project-settings.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-studio.tsx",
        bytes: include_bytes!("templates/pages/platform-project-studio.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-table-connection.tsx",
        bytes: include_bytes!("templates/pages/platform-project-table-connection.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-table-connection-postgresql.tsx",
        bytes: include_bytes!("templates/pages/platform-project-table-connection-postgresql.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-table-connection-sjtable.tsx",
        bytes: include_bytes!("templates/pages/platform-project-table-connection-sjtable.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-tables.tsx",
        bytes: include_bytes!("templates/pages/platform-project-tables.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project.tsx",
        bytes: include_bytes!("templates/pages/platform-project.tsx"),
    },
    EmbeddedAsset {
        path: "styles/main.css",
        bytes: include_bytes!("templates/styles/main.css"),
    },
    EmbeddedAsset {
        path: "styles/db-suite.css",
        bytes: include_bytes!("templates/styles/db-suite.css"),
    },
    EmbeddedAsset {
        path: "styles/db-connections.css",
        bytes: include_bytes!("templates/styles/db-connections.css"),
    },
];

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
