//! Helpers more than one node family uses.
//!
//! - [`file_ref`] — the FileRef payload IR: producers, validators, and the
//!   ZebFS path resolvers every file-taking node goes through.
//! - [`limits`] — closed choices and the ceilings every node shares.
//! - [`project_store`] — which store a node reads or writes, its keys, and
//!   `--on-conflict`.
//! - [`query`] — the database query nodes' `--param`, `--write`, `--limit`
//!   and their `query` answer.
//! - [`store_scratch`] — a local working folder for engines that only speak
//!   file paths: pull from the project's store, run, push back.
//! - [`units`] — durations and sizes in flag values (`30s`, `10MB`).
//! - [`util`] — metadata scope, dot-path lookup and the Deno expression bridge.
//!
//! A helper used by one family lives in that family's folder, not here.

pub mod file_ref;
pub mod limits;
pub mod project_store;
pub mod query;
pub mod store_scratch;
#[cfg(test)]
pub mod test_platform;
pub mod units;
pub mod util;
