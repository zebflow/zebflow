use zebflow::ZebflowEngineKit;

#[test]
fn default_framework_engine_is_registered() {
    let kit = ZebflowEngineKit::with_defaults();
    assert!(kit.framework_engine("framework.noop").is_some());
}
