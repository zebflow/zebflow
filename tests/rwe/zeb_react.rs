use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{
    ReactiveWebEngine, ReactiveWebOptions, RenderContext, RweReactiveWebEngine, TemplateSource,
};

#[test]
fn zeb_react_compiles_and_renders_without_preact() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let source = TemplateSource {
        id: "zeb-react-contract".into(),
        source_path: None,
        markup: r#"
import { createContext, useContext, useId, useState } from "zeb/react";
const Context = createContext("default");
function Field() {
  const id = useId();
  const value = useContext(Context);
  const [count, setCount] = useState(0);
  return <button id={id} aria-expanded={false} style={{width: 12}}
    onClick={() => setCount(n => n + 1)}>{value}:{count}</button>;
}

export default function Page() {
  return <main><Context.Provider value="outer"><Field />
    <Context.Provider value="inner"><Field /></Context.Provider><Field />
    </Context.Provider><Field /></main>;
}
"#
        .into(),
    };
    let compiled = engine
        .compile_template(&source, &language, &ReactiveWebOptions::default())
        .expect("compile Zeb React");
    let context = RenderContext {
        route: "/zeb-react".into(),
        request_id: "zeb-react-contract".into(),
        metadata: json!({}),
        enabled_libraries: vec![],
    };
    for _ in 0..2 {
        let rendered = engine
            .render(&compiled, json!({}), &language, &context)
            .expect("render Zeb React");
        assert!(!rendered.html.contains("RWE component error"));
        for id in 0..4 {
            assert!(rendered.html.contains(&format!("id=\"zeb-{id}\"")));
        }
        assert!(rendered.html.contains("inner:0</button>"));
        assert!(rendered.html.contains("default:0</button>"));
        assert!(rendered.html.contains("aria-expanded=\"false\""));
        assert!(rendered.html.contains("width:12px"));
        let scripts = rendered
            .compiled_scripts
            .iter()
            .map(|script| script.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(scripts.contains("/assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs"));
        assert!(!scripts.contains("/assets/libraries/zeb/preact/"));
    }
}

#[test]
fn react_imports_preserve_aliases_namespaces_and_component_scope() {
    use zebflow::rwe::core::{self, CompileOptions};
    let root = tempfile::tempdir().expect("fixture root");
    std::fs::write(
        root.path().join("counter.tsx"),
        r#"
import React, { useState, useMemo as remember } from "zeb/react";
import * as UI from "zeb/react";
export default function Counter() {
  const [n] = useState(2);
  const [other] = React.useState(3);
  const value = remember(() => n + other, []);
  const hooks = { useState };
  return <b>{value}:{String(hooks.useState === UI.useState)}:{String(UI.default === React)}</b>;
}
"#,
    )
    .expect("component fixture");
    let compiled = core::compile(
        r#"
import { useState as state } from "zeb/react";
import { usePageState } from "zeb";
import Counter from "@/counter";
export default function Page() {
  const [n] = state(7);
  const [label] = usePageState("label", "page");
  return <main>{label}:{n}<Counter /></main>;
}
"#,
        CompileOptions {
            template_root: Some(root.path().to_string_lossy().into()),
            ..CompileOptions::default()
        },
    )
    .expect("compile imports");
    assert!(
        !compiled
            .detected_zeb_libs
            .iter()
            .any(|lib| lib == "zeb/react")
    );
    let output = core::render(&compiled, &json!({}), &["zeb/icons".into()]).expect("SSR aliases");
    assert!(
        !output.html.contains("RWE component error"),
        "{}",
        output.html
    );
    assert!(
        output.html.contains("page:7<b>5:true:true</b>"),
        "{}",
        output.html
    );
}

#[test]
fn react_imports_reject_unsupported_named_exports_in_pages_and_components() {
    use zebflow::rwe::core::{self, CompileOptions};
    for name in ["useTransition", "Suspense", "usePageState"] {
        let source = format!(
            "import {{ {name} }} from 'zeb/react'; export default function Page() {{ return <div />; }}"
        );
        let error =
            core::compile(&source, CompileOptions::default()).expect_err("unsupported export");
        assert_eq!(error.code, "RWE_REACT_EXPORT");
        assert!(error.message.contains(name));
    }
    let root = tempfile::tempdir().expect("fixture root");
    std::fs::write(
        root.path().join("bad.tsx"),
        "import { Suspense } from 'zeb/react'; export default function Bad() { return <div />; }",
    )
    .unwrap();
    let error = core::compile(
        "import Bad from '@/bad'; export default function Page() { return <Bad />; }",
        CompileOptions {
            template_root: Some(root.path().to_string_lossy().into()),
            ..CompileOptions::default()
        },
    )
    .expect_err("unsupported component export");
    assert_eq!(error.code, "RWE_REACT_EXPORT");
}

#[test]
fn recovery_and_store_imports_compile_in_components_and_render_in_v8() {
    use zebflow::rwe::core::{self, CompileOptions};
    let root = tempfile::tempdir().expect("fixture root");
    std::fs::write(
        root.path().join("panel.tsx"),
        r#"
import React, { useSyncExternalStore as useStore } from 'zeb/react';
export default function Panel() {
  const value = useStore(
    () => { throw new Error('SSR subscribed'); },
    () => { throw new Error('SSR read browser store'); },
    () => 'server snapshot'
  );
  return <b>{value}:{String(useStore === React.useSyncExternalStore)}</b>;
}
"#,
    )
    .unwrap();
    let compiled = core::compile(
        r#"
import { ErrorBoundary as Boundary, useSyncExternalStore } from 'zeb/react';
import Panel from '@/panel';
function Broken() { throw new Error('panel failed'); }
function MissingSnapshot() { return useSyncExternalStore(() => () => {}, () => 'browser'); }
export default function Page() {
  return <main><Panel />
    <Boundary fallbackRender={({ error, resetErrorBoundary }) =>
      <button onClick={resetErrorBoundary}>{error.message}</button>}><Broken /></Boundary>
    <Boundary fallback={<i>offline</i>}><MissingSnapshot /></Boundary>
  </main>;
}
"#,
        CompileOptions {
            template_root: Some(root.path().to_string_lossy().into()),
            ..CompileOptions::default()
        },
    )
    .expect("compile recovery APIs");
    for _ in 0..2 {
        let output = core::render(&compiled, &json!({}), &[]).expect("render recovery APIs");
        assert!(
            !output.html.contains("RWE component error"),
            "{}",
            output.html
        );
        assert!(
            output.html.contains("<b>server snapshot:true</b>"),
            "{}",
            output.html
        );
        assert!(
            output.html.contains("<button>panel failed</button>"),
            "{}",
            output.html
        );
        assert!(output.html.contains("<i>offline</i>"), "{}", output.html);
    }
}
