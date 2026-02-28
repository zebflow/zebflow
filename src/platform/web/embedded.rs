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
        path: "components/platform-sidebar.tsx",
        bytes: include_bytes!("templates/components/platform-sidebar.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/button.tsx",
        bytes: include_bytes!("templates/components/ui/button.tsx"),
    },
    EmbeddedAsset {
        path: "components/ui/sonner.tsx",
        bytes: include_bytes!("templates/components/ui/sonner.tsx"),
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
        path: "pages/platform-project-pipelines.tsx",
        bytes: include_bytes!("templates/pages/platform-project-pipelines.tsx"),
    },
    EmbeddedAsset {
        path: "pages/platform-project-section.tsx",
        bytes: include_bytes!("templates/pages/platform-project-section.tsx"),
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
];

pub const PLATFORM_LIBRARY_ASSETS: &[EmbeddedAsset] = &[
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
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs"),
    },
    EmbeddedAsset {
        path: "zeb/codemirror/0.1/wrappers/CodeEditor.tsx",
        bytes: include_bytes!("../../../libraries/zeb/codemirror/0.1/wrappers/CodeEditor.tsx"),
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
];

pub fn platform_library_asset(path: &str) -> Option<&'static [u8]> {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    PLATFORM_LIBRARY_ASSETS
        .iter()
        .find(|asset| asset.path == normalized)
        .map(|asset| asset.bytes)
}
