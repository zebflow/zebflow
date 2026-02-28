use zebflow::ZebflowEngineKit;

#[test]
fn default_language_engines_are_registered() {
    let kit = ZebflowEngineKit::with_defaults();
    assert!(kit.language_engine("language.noop").is_some());
    assert!(kit.language_engine("language.deno_sandbox").is_some());
}
