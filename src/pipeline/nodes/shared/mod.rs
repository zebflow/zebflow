//! Helpers more than one node family uses.
//!
//! - [`file_ref`] — the FileRef payload IR: producers, validators, and the
//!   ZebFS path resolvers every file-taking node goes through.
//! - [`util`] — metadata scope, dot-path lookup and the Deno expression bridge.
//!
//! A helper used by one family lives in that family's folder, not here.

pub mod file_ref;
pub mod util;
