use serde_json::json;
use zebflow::language::DenoSandboxEngine;

/// Guard tick splicing against multi-byte source.
///
/// The instrumenter inserts at AST byte offsets using `String::insert_str`,
/// which is byte-indexed and panics if the index is not a char boundary. Every
/// offset comes from a real parser so it should land on one — but "should" is
/// what tests are for, and a panic here would take down a worker thread on a
/// perfectly ordinary script written in a non-Latin language.
#[test]
fn multibyte_source_does_not_break_tick_splicing() {
    let engine = DenoSandboxEngine::default();
    let cases = [
        ("cjk_identifiers", "const 名前 = '日本語'; let s = 0; \
                             for (const c of 名前) { s += 1; } return { s };"),
        ("emoji_in_string", "const e = '🎉🎉🎉 party'; let s = 0; \
                             while (s < 3) { s += 1; } return { s, len: e.length };"),
        ("emoji_before_loop", "const tag = '→ ✅ ünïcodé'; let s=0; \
                               for (let i=0;i<3;i++){ s+=i; } return { s, tag };"),
        ("rtl_text", "const ar = 'مرحبا بالعالم'; let s=0; \
                      do { s+=1; } while (s < 3); return { s, ar };"),
        ("emoji_braceless_body", "let i=0; const m='🚀'; while(i<3) i++; return { i, m };"),
        ("combining_marks", "const label='é\\u0301ẍ'; let s=0; \
                             for (const k in {a:1,b:2}) { s+=1; } return { s, label };"),
    ];
    for (name, src) in cases {
        let out = engine.run_script(src, &json!({}), None);
        println!("  {name}: {out:?}");
        assert!(out.is_ok(), "multibyte source failed for {name}: {out:?}");
    }
}

/// A body receives `input`, `n` and `ctx` from its wrapper. Redeclaring one is
/// a JavaScript early error, which the parser alone does not catch — it needs
/// scope analysis. Without the semantic pass this reached V8 and died with
/// "Identifier 'n' has already been declared", which says nothing about why
/// `n` was taken.
#[test]
fn colliding_with_a_wrapper_parameter_is_explained_at_compile() {
    let engine = DenoSandboxEngine::default();
    for name in ["input", "n", "ctx"] {
        let src = format!("const {name} = 1; return {{ v: {name} }};");
        let err = engine
            .run_script(&src, &json!({}), None)
            .expect_err("redeclaring a wrapper parameter must be refused");
        println!("  {name}: {}", err.message);
        assert!(
            err.message.contains("cannot declare them again"),
            "unhelpful message for `{name}`: {}",
            err.message
        );
        // The message must name the binding the author actually wrote.
        assert!(
            err.message.contains(&format!("`{name}` is already defined")),
            "message names the wrong identifier for `{name}`: {}",
            err.message
        );
    }
}
