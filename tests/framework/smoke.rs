use zebflow::ZebflowEngineKit;

#[test]
fn default_core_engines_are_registered() {
    let kit = ZebflowEngineKit::with_defaults();
    assert!(kit.pipeline_engine("pipeline.basic").is_some());
    assert!(kit.language_engine("language.deno_sandbox").is_some());
    assert!(kit.rwe_engine("rwe").is_some());
}
