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
import { usePageState } from "zeb/react";
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
    for name in ["useTransition", "Suspense", "useNavigate"] {
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
fn react_platform_search_params_render_in_embedded_v8() {
    use zebflow::rwe::core::{self, CompileOptions};
    let compiled = core::compile(
        r#"
import { useSearchParams, usePathname } from 'zeb/react';
export default function Page() {
  const params = useSearchParams();
  return <main><p>{usePathname()}:{params.get('q')}:{String(params.has('empty'))}:{String(params.get('missing') === null)}</p>
    <b>{params.get(null)}:{params.get(4)}:{params.get(undefined)}</b></main>;
}

"#,
        CompileOptions::default(),
    )
    .expect("compile search hooks");
    let output = core::render(
        &compiled,
        &json!({ "route": "/articles", "query": {
            "q": "café + tea", "empty": "", "null": "null-key", "4": "number-key", "undefined": "undefined-key"
        } }),
        &[],
    )
    .expect("render search hooks in embedded V8");
    assert!(
        output
            .html
            .contains("<p>/articles:café + tea:true:true</p>"),
        "{}",
        output.html
    );
    assert!(
        output
            .html
            .contains("<b>null-key:number-key:undefined-key</b>"),
        "{}",
        output.html
    );
}

#[test]
fn react_platform_search_params_preserve_raw_query_semantics() {
    use zebflow::rwe::core::{self, CompileOptions};
    let compiled = core::compile(
        r#"
import { useSearchParams } from 'zeb/react';
export default function Page() {
  const params = useSearchParams();
  const seen = [];
  params.forEach((value, key, owner) => seen.push(key + ':' + value + ':' + (owner === params)));
  const entries = [...params];
  entries[0][1] = 'modified copy';
  let readonly = false;
  try { params.set('q', 'changed'); } catch (_) { readonly = true; }
  return <main><p>{params.get('q')}|{params.getAll('tag').join(',')}|{params.get('bad')}|{params.get('literal')}|{params.size}</p>
    <b>{String(params.has('tag', 'two'))}:{String(params.has('tag', 'absent'))}:{String(readonly)}</b>
    <i>{[...params.keys()].join(',')}|{[...params.values()].length}|{seen.length}</i>
    <code>{params.toString()}</code></main>;
}
"#,
        CompileOptions::default(),
    )
    .expect("compile raw search hooks");
    let output = core::render(
        &compiled,
        &json!({
            "query": { "q": "must not override raw search" },
            "search": "?q=caf%C3%A9+%2B+tea&tag=one&tag=two&bad=%FF&literal=%ZZ&empty="
        }),
        &[],
    )
    .expect("render raw search hooks");
    assert!(
        output.html.contains("<p>café + tea|one,two|�|%ZZ|6</p>"),
        "{}",
        output.html
    );
    assert!(
        output.html.contains("<b>true:false:true</b>"),
        "{}",
        output.html
    );
    assert!(
        output
            .html
            .contains("<i>q,tag,tag,bad,literal,empty|6|6</i>"),
        "{}",
        output.html
    );
    assert!(output.html.contains("q=caf%C3%A9+%2B+tea&amp;tag=one&amp;tag=two&amp;bad=%EF%BF%BD&amp;literal=%25ZZ&amp;empty="), "{}", output.html);
    assert!(
        output
            .js
            .contains("typeof globalThis.ctx.search === 'string'")
    );
}

#[test]
fn react_platform_pathname_uses_render_context_without_overwriting_payload() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let source = TemplateSource {
        id: "zeb-route-context".into(),
        source_path: None,
        markup: "import { usePathname } from 'zeb/react'; export default function Page() { return <p>{usePathname()}</p>; }".into(),
    };
    let compiled = engine
        .compile_template(&source, &language, &ReactiveWebOptions::default())
        .unwrap();
    let context = RenderContext {
        route: "/context-route".into(),
        request_id: "zeb-route-context".into(),
        metadata: json!({}),
        enabled_libraries: vec![],
    };
    for (state, expected) in [
        (json!({}), "/context-route"),
        (json!({"route": "/explicit"}), "/explicit"),
    ] {
        let output = engine
            .render(&compiled, state, &language, &context)
            .unwrap();
        assert!(
            output.html.contains(&format!("<p>{expected}</p>")),
            "{}",
            output.html
        );
    }
}

#[test]
fn react_platform_namespace_contains_all_named_helpers() {
    use zebflow::rwe::core::{self, CompileOptions};
    let compiled = core::compile(
        r#"
import * as Zeb from 'zeb/react';
import React, { useRouter, usePathname, useSearchParams, usePageState, Link, cx } from 'zeb/react';
export default function Page() {
  const matches = Zeb.useRouter === useRouter && Zeb.usePathname === usePathname
    && Zeb.useSearchParams === useSearchParams && Zeb.usePageState === usePageState
    && Zeb.Link === Link && Zeb.cx === cx && Zeb.default === React;
  return <p>{String(matches)}:{Zeb.usePathname()}</p>;
}
"#,
        CompileOptions::default(),
    )
    .expect("compile platform namespace");
    let output = core::render(&compiled, &json!({ "route": "/namespace" }), &[])
        .expect("render platform namespace");
    assert!(
        output.html.contains("<p>true:/namespace</p>"),
        "{}",
        output.html
    );
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
