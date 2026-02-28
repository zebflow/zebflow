use zebflow::ZebflowEngineKit;

#[test]
fn default_rwe_engine_is_registered() {
    let kit = ZebflowEngineKit::with_defaults();
    assert!(kit.rwe_engine("rwe.noop").is_some());
}
