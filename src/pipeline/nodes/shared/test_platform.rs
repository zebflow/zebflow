//! A platform on a temporary data root, for tests.
//!
//! The folder lives exactly as long as the value: dropping it at the end of a
//! test removes everything the test wrote. A helper that kept the folder
//! (`tempdir().keep()`) left one data root behind per test run, gigabytes a day.

use std::ops::Deref;
use std::sync::Arc;

use crate::platform::services::PlatformService;

pub struct TestPlatform {
    platform: Arc<PlatformService>,
    _root: tempfile::TempDir,
}

impl Deref for TestPlatform {
    type Target = Arc<PlatformService>;

    fn deref(&self) -> &Self::Target {
        &self.platform
    }
}

/// A fresh platform whose data root is removed when the value is dropped.
pub fn test_platform() -> TestPlatform {
    let root = tempfile::tempdir().expect("tempdir");
    let mut config = crate::platform::model::PlatformConfig::default();
    config.data_root = root.path().join("platform");
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    TestPlatform { platform, _root: root }
}
